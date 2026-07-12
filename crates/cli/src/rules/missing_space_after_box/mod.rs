//! E005 — flags tasks/subtasks missing a space after the status box.

use crate::parser::{FileItem, ParsingIssue};
use crate::rules::{Issue, for_each_node};

/// Flags tasks/subtasks that are missing a space between `[status]` and title.
/// Valid: `- [ ] title` or `- [x] title`
/// Invalid: `- [ ]title` or `- [x]title`
pub fn missing_space_after_box(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        if node
            .parsing_issues()
            .contains(&ParsingIssue::MissingSpaceAfterBox)
        {
            issues.push(Issue {
                location: node.location().clone(),
                code: crate::rules::ErrorCode::MissingSpaceAfterBox,
                message: "Missing space after status box".to_string(),
                // Position right after the `]` in `- [x]`, relative to indent.
                column: node.indent() + 6,
                help: Some(
                    "Add a space between the status box and the task title: `- [ ] title`"
                        .to_string(),
                ),
                data: Some(crate::rules::IssueData::MissingSpaceAfterBox),
            });
        }
    });
    issues
}

#[cfg(test)]
mod tests;
