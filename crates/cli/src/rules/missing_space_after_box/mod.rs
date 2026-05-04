//! E005 — flags tasks/subtasks missing a space after the status box.

use crate::parser::{FileItem, ParsingIssue, Subtask};
use crate::rules::Issue;

/// Flags tasks/subtasks that are missing a space between `[status]` and title.
/// Valid: `- [ ] title` or `- [x] title`
/// Invalid: `- [ ]title` or `- [x]title`
pub fn missing_space_after_box(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for item in items {
        if let FileItem::Task(task) = item {
            if task
                .parsing_issues
                .contains(&ParsingIssue::MissingSpaceAfterBox)
            {
                issues.push(Issue {
                    location: task.location.clone(),
                    code: crate::rules::ErrorCode::MissingSpaceAfterBox,
                    message: "Missing space after status box".to_string(),
                    column: 6, // Position right after the `]` in `- [x]`
                    help: Some(
                        "Add a space between the status box and the task title: `- [ ] title`"
                            .to_string(),
                    ),
                    data: Some(crate::rules::IssueData::MissingSpaceAfterBox),
                });
            }
            issues.extend(check_subtasks(&task.children));
        }
    }

    issues
}

fn check_subtasks(subtasks: &[Subtask]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for subtask in subtasks {
        if subtask
            .parsing_issues
            .contains(&ParsingIssue::MissingSpaceAfterBox)
        {
            issues.push(Issue {
                location: subtask.location.clone(),
                code: crate::rules::ErrorCode::MissingSpaceAfterBox,
                message: "Missing space after status box".to_string(),
                column: subtask.indent + 6, // Position right after the `]` relative to indent
                help: Some(
                    "Add a space between the status box and the task title: `- [ ] title`"
                        .to_string(),
                ),
                data: Some(crate::rules::IssueData::MissingSpaceAfterBox),
            });
        }
        issues.extend(check_subtasks(&subtask.children));
    }

    issues
}

#[cfg(test)]
mod tests;
