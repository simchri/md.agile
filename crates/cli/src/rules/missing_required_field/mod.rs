//! E024 — flags a task/subtask whose property attaches a form (see
//! README.vision.md "Forms & Fields") but is missing one or more of that
//! form's required fields entirely (no matching `__Label:__ value` line at
//! all in its body).
//!
//! This only checks *presence* of a field line. Whether a present field is
//! left unfilled (still the placeholder value -- see
//! [`crate::rules::field_left_unfilled`], E026) or fails its
//! regex/allowed-values validation (not yet implemented) are separate
//! checks.
//!
//! A label with a *malformed* field line (missing the space before its
//! value, e.g. `__Email:__john@example.com` -- see
//! [`crate::rules::missing_space_after_field_label`], E025) is excluded from
//! "missing" here: the real problem is the missing space, not an absent
//! field, and E025 already reports it with a more specific, actionable
//! message. Reporting both would be redundant and confusing.
//!
//! A field with [`crate::config::FieldConfig::optional`] set is excluded
//! entirely: it's never reported missing, whether or not its body line is
//! present at all.

use crate::config::Config;
use crate::parser::Marker;
use crate::rules::{ErrorCode, Issue, IssueData, for_each_node};

pub fn missing_required_field(items: &[crate::parser::FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
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
                // Undefined form reference is a separate config-validation
                // check's concern -- not reported here.
                continue;
            };

            let present_labels: Vec<&str> =
                node.fields().iter().map(|f| f.label.as_str()).collect();
            let malformed_labels: Vec<&str> = node
                .malformed_fields()
                .iter()
                .map(|f| f.label.as_str())
                .collect();

            let missing: Vec<String> = form_config
                .fields
                .iter()
                .filter_map(|field_name| config.fields.get(field_name))
                .filter(|field_config| !field_config.optional)
                .filter(|field_config| !present_labels.contains(&field_config.label.as_str()))
                .filter(|field_config| !malformed_labels.contains(&field_config.label.as_str()))
                .map(|field_config| field_config.label.clone())
                .collect();

            if !missing.is_empty() {
                let missing_quoted: Vec<String> =
                    missing.iter().map(|s| format!("\"{s}\"")).collect();
                issues.push(Issue {
                    location: node.location().clone(),
                    code: ErrorCode::MissingRequiredField,
                    message: format!(
                        "Missing required field{}: {}",
                        if missing.len() == 1 { "" } else { "s" },
                        missing_quoted.join(", "),
                    ),
                    column: node.indent() + 1,
                    help: Some(
                        "Add the missing field line(s), e.g. `__Label:__ value`".to_string(),
                    ),
                    data: Some(IssueData::MissingRequiredField { missing }),
                });
            }
        }
    });
    issues
}

#[cfg(test)]
mod tests;
