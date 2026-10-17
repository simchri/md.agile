//! E030 / E031 — runs each form's configured `validation` script (see
//! `doc/usage/form_validation_scripts.md`) against every task or subtask that
//! carries the form, and turns the verdict into issues:
//!
//! - E030 when the script ran and reported the form invalid, anchored at the
//!   offending field line (or the task line when no field is named / present);
//! - E031 when the script could not be run properly (untrusted, missing,
//!   crashed, timed out, bad output), anchored at the task line.
//!
//! This is deliberately not part of [`crate::rules::check_all`]: it executes
//! external programs and needs the project root and trust store, so callers
//! opt in explicitly (`agile check` does; the LSP does not run it on edits).

use crate::config::{Config, FormConfig};
use crate::form_script::{
    Limits, RunError, ScriptError, ScriptInput, ScriptOutput, TaskInfo, run_script,
};
use crate::parser::{FileItem, Marker};
use crate::rules::{ErrorCode, Issue, IssueData, NodeRef, for_each_node};
use crate::trust::{TrustStore, user_store_path};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Like [`check_form_scripts`], using the current user's trust store (a
/// missing or unreadable store means nothing is trusted).
pub fn check_form_scripts_default(items: &[FileItem], root: &Path, config: &Config) -> Vec<Issue> {
    if !config.forms.values().any(|f| f.validation.is_some()) {
        return vec![];
    }
    let store = user_store_path()
        .map(|path| TrustStore::load(&path))
        .transpose()
        .unwrap_or_else(|e| {
            log::warn!("{e} — treating every form validation script as untrusted");
            None
        })
        .unwrap_or_default();
    check_form_scripts(items, root, config, &store, &Limits::default())
}

pub fn check_form_scripts(
    items: &[FileItem],
    root: &Path,
    config: &Config,
    store: &TrustStore,
    limits: &Limits,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        let mut seen = BTreeSet::new();
        for marker in node.markers() {
            let Marker::Property(prop) = marker else {
                continue;
            };
            let Some(form_name) = config
                .properties
                .get(&prop.name)
                .and_then(|p| p.form.as_ref())
            else {
                continue;
            };
            let Some(form) = config.forms.get(form_name) else {
                continue;
            };
            let Some(script) = &form.validation else {
                continue;
            };
            if !seen.insert(form_name.clone()) {
                continue;
            }

            let form_fields = form_fields(form, config);
            let input = build_script_input(node, form_name, &form_fields);
            let result = run_script(root, script, &input, store, limits);
            issues.extend(verdict_issues(
                node,
                form_name,
                script,
                &form_fields,
                result,
            ));
        }
    });
    issues
}

/// (toml key, display label) of each field of the form.
fn form_fields<'a>(form: &'a FormConfig, config: &'a Config) -> Vec<(&'a str, &'a str)> {
    form.fields
        .iter()
        .filter_map(|key| {
            config
                .fields
                .get(key)
                .map(|f| (key.as_str(), f.label.as_str()))
        })
        .collect()
}

/// The data sent to the script: the form's own fields, keyed by toml key.
fn build_script_input(node: NodeRef, form_name: &str, form_fields: &[(&str, &str)]) -> ScriptInput {
    let fields: BTreeMap<String, String> = form_fields
        .iter()
        .filter_map(|(key, label)| {
            node.fields()
                .iter()
                .find(|line| line.label == *label)
                .map(|line| (key.to_string(), line.value.trim().to_string()))
        })
        .collect();
    ScriptInput {
        form: form_name.to_string(),
        task: TaskInfo {
            title: node.title().to_string(),
            line: node.location().line,
        },
        fields,
    }
}

/// Turns the outcome of one script run into issues: none when valid, E030
/// per reported error when invalid, E031 when the script could not run.
fn verdict_issues(
    node: NodeRef,
    form_name: &str,
    script: &str,
    form_fields: &[(&str, &str)],
    result: Result<ScriptOutput, RunError>,
) -> Vec<Issue> {
    match result {
        Ok(output) if output.valid => vec![],
        Ok(output) => {
            let errors = if output.errors.is_empty() {
                vec![ScriptError {
                    field: None,
                    message: "the form is not valid".to_string(),
                }]
            } else {
                output.errors
            };
            errors
                .into_iter()
                .map(|error| validation_failed(node, form_name, form_fields, error))
                .collect()
        }
        Err(error) => vec![script_failed(node, form_name, script, &error)],
    }
}

/// `form_fields` maps the form's toml keys to display labels; the script
/// names fields by key.
fn validation_failed(
    node: NodeRef,
    form: &str,
    form_fields: &[(&str, &str)],
    error: ScriptError,
) -> Issue {
    let label_of_field = error.field.as_ref().and_then(|key| {
        form_fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, label)| *label)
    });
    let line_of_field = label_of_field.and_then(|label| {
        node.fields()
            .iter()
            .find(|f| f.label == label)
            .map(|f| f.location.clone())
    });
    let location = line_of_field.unwrap_or_else(|| node.location().clone());
    let message = match &error.field {
        Some(key) => format!(
            "Field \"{}\" is invalid: {}",
            label_of_field.unwrap_or(key),
            error.message
        ),
        None => format!("Form \"{form}\" is invalid: {}", error.message),
    };
    Issue {
        location,
        code: ErrorCode::FormValidationFailed,
        message,
        column: 1,
        help: None,
        data: Some(IssueData::FormValidationFailed {
            form: form.to_string(),
            field: error.field,
            message: error.message,
        }),
    }
}

fn script_failed(node: NodeRef, form: &str, script: &str, error: &RunError) -> Issue {
    let help = match error {
        RunError::Untrusted => Some(format!(
            "Review `{script}`, then approve it with `agile trust`. \
             On a CI runner use `agile trust --yes` before `agile check`."
        )),
        _ => None,
    };
    Issue {
        location: node.location().clone(),
        code: ErrorCode::FormScriptFailed,
        message: format!(
            "Validation script \"{script}\" of form \"{form}\" could not run: {error}"
        ),
        column: 1,
        help,
        data: Some(IssueData::FormScriptFailed {
            form: form.to_string(),
            script: script.to_string(),
        }),
    }
}

#[cfg(test)]
#[path = "form_validation_tests.rs"]
mod tests;
