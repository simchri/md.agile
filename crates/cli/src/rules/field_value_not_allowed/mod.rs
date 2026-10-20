//! E029 — flags a present form field whose value doesn't belong to its
//! configured closed set of allowed [`crate::config::FieldConfig::values`]
//! (see README.vision.md "Forms & Fields"), e.g. `__Gender:__ nonbinary`
//! when the field only allows `["male", "female", "(don't specify)"]`.
//!
//! This only judges an actual, deliberately entered value -- a field that's
//! blank, or still equals its resolved "unfilled placeholder" (the
//! configured `default`, or the generic `___` when it has none), is
//! [`crate::rules::field_left_unfilled`]'s concern (E026), not this rule's:
//! it hasn't been filled out yet at all, so there's nothing to validate
//! against the allowed set. A field with no configured `values` is never
//! checked here.
//!
//! Like E028, this is a validity mistake independent of whether the task is
//! open or done -- it's wrong the moment it's written -- so it's not gated
//! on [`crate::parser::Status::Done`].
//!
//! A field line that's entirely absent is
//! [`crate::rules::missing_required_field`]'s concern (E024), not this
//! rule's. Likewise a *malformed* field line (missing the space before its
//! value -- see [`crate::rules::missing_space_after_field_label`], E025)
//! has no parsed value at all, so it can't be checked here either.

use crate::config::Config;
use crate::rules::{ErrorCode, Issue, IssueData, for_each_node};

pub fn field_value_not_allowed(items: &[crate::parser::FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        for form in config.active_forms(node.markers()) {
            let form_config = form.config;

            for field_name in &form_config.fields {
                let Some(field_config) = config.fields.get(field_name) else {
                    continue;
                };
                let Some(allowed) = &field_config.values else {
                    continue;
                };
                let Some(field_line) = node.fields().iter().find(|f| f.label == field_config.label)
                else {
                    continue;
                };

                let trimmed_value = field_line.value.trim();
                if trimmed_value.is_empty() {
                    continue;
                }
                let placeholder = field_config.default.as_deref().unwrap_or("___");
                if trimmed_value == placeholder.trim() {
                    continue;
                }
                if allowed.iter().any(|v| v == trimmed_value) {
                    continue;
                }

                issues.push(Issue {
                    location: field_line.location.clone(),
                    code: ErrorCode::FieldValueNotAllowed,
                    message: format!(
                        "Field \"{}\" value \"{}\" is not one of the allowed values ({})",
                        field_config.label,
                        trimmed_value,
                        allowed.join(", ")
                    ),
                    column: 1,
                    help: Some(format!("Use one of: {}", allowed.join(", "))),
                    data: Some(IssueData::FieldValueNotAllowed {
                        label: field_config.label.clone(),
                        value: trimmed_value.to_string(),
                        allowed: allowed.clone(),
                    }),
                });
            }
        }
    });
    issues
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
