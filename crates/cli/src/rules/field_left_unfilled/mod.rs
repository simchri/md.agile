//! E026 — flags a task/subtask whose property attaches a form (see
//! README.vision.md "Forms & Fields") and *has* a matching `__Label:__
//! value` line for one of that form's fields, but whose value still equals
//! the field's resolved "unfilled placeholder" -- i.e. nobody has actually
//! filled it out yet.
//!
//! The resolved placeholder is the field's configured `default` (see
//! [`crate::config::FieldConfig::default`]), or the generic `___` when it
//! has none -- the same value the "insert missing fields" quickfix
//! ([`crate::lsp::quickfix::missing_required_field`]) inserts for a brand
//! new field line. This mirrors that module's terminology: a field counts
//! as "unfilled" whenever it still carries exactly what would have been
//! auto-inserted, custom default included.
//!
//! A field line that's entirely absent is [`crate::rules::missing_required_field`]'s
//! concern (E024), not this rule's -- this only looks at fields that *are*
//! present. Likewise a *malformed* field line (missing the space before its
//! value -- see [`crate::rules::missing_space_after_field_label`], E025) has
//! no parsed value at all, so it can't be checked here either.

use crate::config::Config;
use crate::parser::Marker;
use crate::rules::{ErrorCode, Issue, IssueData, for_each_node};

pub fn field_left_unfilled(items: &[crate::parser::FileItem], config: &Config) -> Vec<Issue> {
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

            let unfilled: Vec<String> = form_config
                .fields
                .iter()
                .filter_map(|field_name| config.fields.get(field_name))
                .filter_map(|field_config| {
                    let placeholder = field_config.default.as_deref().unwrap_or("___");
                    node.fields()
                        .iter()
                        .find(|f| f.label == field_config.label)
                        .filter(|f| f.value == placeholder)
                        .map(|_| field_config.label.clone())
                })
                .collect();

            if !unfilled.is_empty() {
                let unfilled_quoted: Vec<String> =
                    unfilled.iter().map(|s| format!("\"{s}\"")).collect();
                issues.push(Issue {
                    location: node.location().clone(),
                    code: ErrorCode::FieldLeftUnfilled,
                    message: format!(
                        "Field{} left unfilled: {}",
                        if unfilled.len() == 1 { "" } else { "s" },
                        unfilled_quoted.join(", "),
                    ),
                    column: node.indent() + 1,
                    help: Some("Fill in the field's value".to_string()),
                    data: Some(IssueData::FieldLeftUnfilled { unfilled }),
                });
            }
        }
    });
    issues
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
