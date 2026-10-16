//! E024 — flags a task/subtask whose property attaches a form (see
//! README.vision.md "Forms & Fields") but is missing one or more of that
//! form's required fields entirely (no matching `__Label:__ value` line at
//! all in its body).
//!
//! This only checks *presence* of a field line. Whether a present field is
//! left unfilled (still the placeholder value) or fails its
//! regex/allowed-values validation are separate, not-yet-implemented checks.

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

            let missing: Vec<String> = form_config
                .fields
                .iter()
                .filter_map(|field_name| config.fields.get(field_name))
                .filter(|field_config| !present_labels.contains(&field_config.label.as_str()))
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
