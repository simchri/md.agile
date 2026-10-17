//! E027 — flags a task/subtask on which two *distinct* properties each
//! attach a `[Form.X]` (see README.vision.md "Forms & Fields"), when those
//! two forms declare a field with the same display label.
//!
//! A single `__Label:__ value` body line can only ever record one answer,
//! so if two unrelated forms both expect a field labeled e.g. `Name`, the
//! one line on the task could never unambiguously satisfy both at once --
//! this is disallowed outright rather than silently letting one line
//! "double up" for both.
//!
//! Two properties attaching the *same* form are not a collision -- there's
//! only one set of field labels in play, not two forms disagreeing about
//! what a label means. And a label collision *within* a single form is
//! already rejected at config-load time (see
//! [`crate::config::ConfigError::FormValidation`]), so it can never reach
//! this rule at all.

use crate::config::Config;
use crate::parser::Marker;
use crate::rules::{ErrorCode, Issue, IssueData, for_each_node};

pub fn colliding_field_labels(items: &[crate::parser::FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        // label -> (form name, property name) for every field declared by
        // every form attached to this node.
        let mut owners: Vec<(&str, &str, &str)> = Vec::new();
        for marker in node.markers() {
            let Marker::Property(prop) = marker else {
                continue;
            };
            let Some(prop_config) = config.properties.get(&prop.name) else {
                continue;
            };
            let Some(form_name) = &prop_config.form else {
                continue;
            };
            let Some(form_config) = config.forms.get(form_name) else {
                continue;
            };
            for field_name in &form_config.fields {
                let Some(field_config) = config.fields.get(field_name) else {
                    continue;
                };
                owners.push((field_config.label.as_str(), form_name.as_str(), &prop.name));
            }
        }

        let mut labels: Vec<&str> = owners.iter().map(|(label, _, _)| *label).collect();
        labels.sort_unstable();
        labels.dedup();

        for label in labels {
            let mut forms: Vec<&str> = owners
                .iter()
                .filter(|(l, _, _)| *l == label)
                .map(|(_, form, _)| *form)
                .collect();
            forms.sort_unstable();
            forms.dedup();
            if forms.len() < 2 {
                // Either only one form declares this label, or every
                // property declaring it shares the same form -- not a
                // collision either way.
                continue;
            }

            let mut properties: Vec<String> = owners
                .iter()
                .filter(|(l, _, _)| *l == label)
                .map(|(_, _, prop)| prop.to_string())
                .collect();
            properties.sort();
            properties.dedup();

            issues.push(Issue {
                location: node.location().clone(),
                code: ErrorCode::CollidingFieldLabels,
                message: format!(
                    "Colliding field label \"{label}\": properties {} each attach a form declaring a field with this label",
                    properties
                        .iter()
                        .map(|p| format!("\"{p}\""))
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
                column: node.indent() + 1,
                help: Some(
                    "Rename one of the colliding fields' labels so each form's fields are unambiguous".to_string(),
                ),
                data: Some(IssueData::CollidingFieldLabels {
                    label: label.to_string(),
                    properties,
                }),
            });
        }
    });
    issues
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
