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
//! auto-inserted, custom default included -- unless
//! [`crate::config::FieldConfig::default_is_valid_entry`] opts the field out
//! of this check entirely by treating its placeholder as a legitimate,
//! deliberately chosen value (e.g. a default like `"(don't specify)"` that's
//! itself one of the field's allowed `values`).
//!
//! A field line that's entirely absent is [`crate::rules::missing_required_field`]'s
//! concern (E024), not this rule's -- this only looks at fields that *are*
//! present. Likewise a *malformed* field line (missing the space before its
//! value -- see [`crate::rules::missing_space_after_field_label`], E025) has
//! no parsed value at all, so it can't be checked here either.
//!
//! Unlike E024, this check only matters at the moment a task is (about to
//! be) marked done -- a form field is allowed to sit unfilled for as long
//! as the task stays open. So, same as [`crate::rules::incomplete_parent`]
//! (E004), it's gated on `Status::Done` in the full-project scan here (to
//! retroactively catch an already-done task whose field was never filled
//! in), and reused unconditionally via [`check_node`] by
//! [`crate::rules::check_completable`], which validates a node *before* the
//! write that would mark it done.

use crate::config::Config;
use crate::parser::{FieldLine, Marker, Status};
use crate::rules::{ErrorCode, Issue, IssueData, for_each_node};

pub fn field_left_unfilled(items: &[crate::parser::FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        if *node.status() != Status::Done {
            return;
        }
        issues.extend(check_node(
            node.markers(),
            node.fields(),
            node.location(),
            node.indent(),
            config,
        ));
    });
    issues
}

/// Checks a single node's `markers`/`fields` for form fields left unfilled
/// (E026). Exposed at `pub(crate)` so `agile task done <address>` can reuse
/// this exact rule, via [`crate::rules::check_completable`], to validate a
/// single addressed node *before* it's marked done, without duplicating the
/// logic or running the full rule over an entire project.
pub(crate) fn check_node(
    markers: &[Marker],
    fields: &[FieldLine],
    location: &crate::parser::Location,
    indent: usize,
    config: &Config,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    for marker in markers {
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

        // Several `[Field.X]` entries may share one display `label` so that
        // a single physical `__Label:__ value` line satisfies all of them
        // (see `missing_required_field`'s label-aliasing support). Track
        // already-reported labels so that aliasing doesn't surface the same
        // unfilled line more than once.
        let mut seen_labels = std::collections::HashSet::new();
        let unfilled: Vec<String> = form_config
            .fields
            .iter()
            .filter_map(|field_name| config.fields.get(field_name))
            .filter(|field_config| !field_config.default_is_valid_entry)
            .filter_map(|field_config| {
                let placeholder = field_config.default.as_deref().unwrap_or("___");
                fields
                    .iter()
                    .find(|f| f.label == field_config.label)
                    .filter(|f| f.value == placeholder)
                    .map(|_| field_config.label.clone())
            })
            .filter(|label| seen_labels.insert(label.clone()))
            .collect();

        if !unfilled.is_empty() {
            let unfilled_quoted: Vec<String> =
                unfilled.iter().map(|s| format!("\"{s}\"")).collect();
            issues.push(Issue {
                location: location.clone(),
                code: ErrorCode::FieldLeftUnfilled,
                message: format!(
                    "Field{} left unfilled: {}",
                    if unfilled.len() == 1 { "" } else { "s" },
                    unfilled_quoted.join(", "),
                ),
                column: indent + 1,
                help: Some("Fill in the field's value".to_string()),
                data: Some(IssueData::FieldLeftUnfilled { unfilled }),
            });
        }
    }
    issues
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
