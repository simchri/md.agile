//! E025 — flags a form field label line that's missing the required space
//! before its value, e.g. `__Email:__john@example.com` instead of
//! `__Email:__ john@example.com`.
//!
//! This is purely structural (no [`crate::config::Config`] needed) and, in
//! `check_all`, is registered to run *before* E024 "Missing Required Field"
//! ([`crate::rules::missing_required_field`]) — which also excludes any
//! label flagged here from its own "missing" computation, since the real
//! problem is the missing space, not an absent field.

use crate::parser::FileItem;
use crate::rules::{ErrorCode, Issue, IssueData, for_each_node};

pub fn missing_space_after_field_label(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        for malformed in node.malformed_fields() {
            issues.push(Issue {
                location: malformed.location.clone(),
                code: ErrorCode::MissingSpaceAfterFieldLabel,
                message: format!(
                    "Missing space after field label: `__{}:__` must be followed by a space before its value",
                    malformed.label
                ),
                column: 1,
                help: Some(format!("Add a space: `__{}:__ value`", malformed.label)),
                data: Some(IssueData::MissingSpaceAfterFieldLabel {
                    label: malformed.label.clone(),
                }),
            });
        }
    });
    issues
}

#[cfg(test)]
mod tests;
