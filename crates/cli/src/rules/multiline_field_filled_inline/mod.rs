//! E028 — flags a form field that's configured as multiline (see
//! [`crate::config::FieldConfig::multiline`]) but was actually written
//! using the single-line (inline) grammar, e.g. `__Bio:__ Grew up in
//! Portland.` instead of opening the label on its own line and putting the
//! content on the line(s) below it.
//!
//! A multiline field's opening `__Label:__` line must have nothing after it
//! -- not even trailing whitespace, since any trailing character there
//! makes [`crate::parser::parse_single_line_field`] match first and claim
//! the line as single-line, regardless of whether the resulting value is
//! itself blank. So this rule only needs to check the field's actual
//! recorded [`crate::parser::FieldLine::multiline`] flag against the
//! field's configured one -- it never has to re-inspect the raw line text.
//!
//! This is a structural/authoring mismatch, independent of whether the
//! task is open or done -- it's wrong the moment it's written -- so, like
//! E024/E025/E027, it's not gated on [`crate::parser::Status::Done`].
//!
//! A field line that's entirely absent is
//! [`crate::rules::missing_required_field`]'s concern (E024), not this
//! rule's. Likewise a *malformed* field line (missing the space before its
//! value -- see [`crate::rules::missing_space_after_field_label`], E025)
//! has no parsed value at all, so it can't be checked here either. And
//! whether a present field's value is blank or still at its placeholder is
//! [`crate::rules::field_left_unfilled`]'s concern (E026), not this one --
//! this rule only cares about *which grammar* was used to write the field,
//! not its content.

use crate::config::Config;
use crate::rules::{ErrorCode, Issue, IssueData, for_each_node};

pub fn multiline_field_filled_inline(
    items: &[crate::parser::FileItem],
    config: &Config,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        for form in config.active_forms(node.markers()) {
            let form_config = form.config;

            for field_name in &form_config.fields {
                let Some(field_config) = config.fields.get(field_name) else {
                    continue;
                };
                if !field_config.multiline {
                    continue;
                }
                let Some(field_line) = node.fields().iter().find(|f| f.label == field_config.label)
                else {
                    continue;
                };
                if field_line.multiline {
                    continue;
                }

                issues.push(Issue {
                    location: field_line.location.clone(),
                    code: ErrorCode::MultilineFieldFilledInline,
                    message: format!(
                        "Field \"{}\" is configured as multiline but was filled out inline",
                        field_config.label
                    ),
                    column: 1,
                    help: Some(
                        "Put the field's value on the line(s) below its label, not directly after it".to_string(),
                    ),
                    data: Some(IssueData::MultilineFieldFilledInline {
                        label: field_config.label.clone(),
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
