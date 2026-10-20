//! E026 — flags a task/subtask whose property attaches a form (see
//! README.vision.md "Forms & Fields") and *has* a matching `__Label:__
//! value` line for one of that form's fields, but whose value is still
//! blank, or still equals the field's resolved "unfilled placeholder" --
//! i.e. nobody has actually filled it out yet.
//!
//! The resolved placeholder is the field's configured `default` (see
//! [`crate::config::FieldConfig::default`]), or the generic `___` when it
//! has none -- the same value the "insert missing fields" quickfix
//! ([`crate::lsp::quickfix::missing_required_field`]) inserts for a brand
//! new field line. This mirrors that module's terminology: a field counts
//! as "unfilled" whenever it still carries exactly what would have been
//! auto-inserted, custom default included -- unless
//! [`crate::config::FieldConfig::default_is_valid_entry`] treats that
//! placeholder as a legitimate, deliberately chosen value instead (e.g. a
//! default like `"(don't specify)"` that's itself one of the field's
//! allowed `values`). That flag only reclassifies the placeholder itself --
//! it does *not* disable this check: a field left entirely blank (no value
//! at all after the label) is never a legitimate answer and is always
//! flagged, with or without the flag.
//!
//! The issue's `help` text -- shown alongside the error message in the LSP
//! -- uses the field's configured [`crate::config::FieldConfig::help`]
//! hint when set (e.g. "Use the person's legal first name"), falling back
//! to a generic "Fill in the field's value" when the field has no such
//! hint configured.
//!
//! A field line that's entirely absent is [`crate::rules::missing_required_field`]'s
//! concern (E024), not this rule's -- this only looks at fields that *are*
//! present. Likewise a *malformed* field line (missing the space before its
//! value -- see [`crate::rules::missing_space_after_field_label`], E025) has
//! no parsed value at all, so it can't be checked here either.
//!
//! This applies equally to multiline fields (see [`crate::config::FieldConfig::multiline`]):
//! a multiline field whose captured content is empty or holds only blank
//! lines resolves to the empty string, same as a blank single-line value,
//! so it's flagged the same way -- "blank" here means "no real content",
//! not "no text on the opening `__Label:__` line".
//!
//! Each unfilled field gets its own issue, anchored at that field's own
//! `__Label:__` line (same as E025/E028), rather than one issue per
//! property aggregating every unfilled field at the task/subtask line --
//! each field line is where the fix actually has to happen.
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
        issues.extend(check_node(node.markers(), node.fields(), config));
    });
    issues
}

/// Checks a single node's `markers`/`fields` for form fields left unfilled
/// (E026). Exposed at `pub(crate)` so `agile task done <address>` can reuse
/// this exact rule, via [`crate::rules::check_completable`], to validate a
/// single addressed node *before* it's marked done, without duplicating the
/// logic or running the full rule over an entire project.
pub(crate) fn check_node(markers: &[Marker], fields: &[FieldLine], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for form in config.active_forms(markers) {
        let form_config = form.config;

        // Field labels are guaranteed unique within a single form (two
        // `[Field.X]` entries resolving to the same label inside one
        // `[Form.X]` is rejected outright at config-load time -- see
        // `crate::config::ConfigError::FormValidation` -- and a cross-form
        // collision, where *two different* forms on the same node share a
        // label, is `crate::rules::colliding_field_labels`'s concern (E027),
        // not this rule's), so no dedup is needed here: each iteration below
        // reports a distinct label.
        for field_config in form_config
            .fields
            .iter()
            .filter_map(|field_name| config.fields.get(field_name))
        {
            let Some(field_line) = fields.iter().find(|f| f.label == field_config.label) else {
                continue;
            };
            let placeholder = field_config.default.as_deref().unwrap_or("___");
            // A field left entirely blank is never a legitimate answer,
            // regardless of `default_is_valid_entry` -- that flag only
            // says the *placeholder itself* counts as a deliberately
            // chosen value, not that the check is disabled outright.
            // Both sides are trimmed before comparing so incidental
            // whitespace around the placeholder (e.g. a stray trailing
            // space) doesn't let a still-unfilled value slip through.
            let trimmed_value = field_line.value.trim();
            let is_blank = trimmed_value.is_empty();
            let still_at_placeholder =
                !field_config.default_is_valid_entry && trimmed_value == placeholder.trim();
            if !(is_blank || still_at_placeholder) {
                continue;
            }

            issues.push(Issue {
                location: field_line.location.clone(),
                code: ErrorCode::FieldLeftUnfilled,
                message: format!("Field \"{}\" left unfilled", field_config.label),
                column: 1,
                help: Some(
                    field_config
                        .help
                        .clone()
                        .unwrap_or_else(|| "Fill in the field's value".to_string()),
                ),
                data: Some(IssueData::FieldLeftUnfilled {
                    label: field_config.label.clone(),
                }),
            });
        }
    }
    issues
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
