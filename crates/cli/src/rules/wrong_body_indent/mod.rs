//! E003 — flags task/subtask description lines with misaligned indentation.

use crate::parser::{FileItem, Subtask};
use crate::rules::Issue;

/// Flags task and subtask body lines that don't match the expected indentation.
///
/// For a task/subtask at indentation level `indent`, body lines should be
/// indented to exactly `indent + 2` spaces. Lines that deviate from this
/// are flagged as wrong body indent.
pub fn wrong_body_indent(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for item in items {
        if let FileItem::Task(task) = item {
            issues.extend(check_task_body(task));
        }
    }

    issues
}

fn check_task_body(task: &crate::parser::Task) -> Vec<Issue> {
    let mut issues = Vec::new();
    let expected_indent = task.indent + 2;

    // Check body lines
    for (idx, line) in task.body.iter().enumerate() {
        let actual_indent = line.chars().take_while(|c| *c == ' ').count();
        if actual_indent != expected_indent && !line.trim().is_empty() {
            // Body lines come after the task title line
            let body_line_number = task.location.line + 1 + idx;
            issues.push(Issue {
                location: crate::parser::Location {
                    path: task.location.path.clone(),
                    line: body_line_number,
                },
                code: crate::rules::ErrorCode::WrongBodyIndentation,
                message: "Wrong body indentation".to_string(),
                column: actual_indent + 1,
                help: Some(format!(
                    "Body lines should be indented to {} space{}, but this line has {}.",
                    expected_indent,
                    if expected_indent == 1 { "" } else { "s" },
                    actual_indent
                )),
                data: Some(crate::rules::IssueData::WrongBodyIndent { expected_indent }),
            });
        }
    }

    // Check subtask body lines recursively
    for subtask in &task.children {
        issues.extend(check_subtask_body(subtask));
    }

    issues
}

fn check_subtask_body(subtask: &Subtask) -> Vec<Issue> {
    let mut issues = Vec::new();
    let expected_indent = subtask.indent + 2;

    for (idx, line) in subtask.body.iter().enumerate() {
        let actual_indent = line.chars().take_while(|c| *c == ' ').count();
        if actual_indent != expected_indent && !line.trim().is_empty() {
            let body_line_number = subtask.location.line + 1 + idx;
            issues.push(Issue {
                location: crate::parser::Location {
                    path: subtask.location.path.clone(),
                    line: body_line_number,
                },
                code: crate::rules::ErrorCode::WrongBodyIndentation,
                message: "Wrong body indentation".to_string(),
                column: actual_indent + 1,
                help: Some(format!(
                    "Body lines should be indented to {} space{}, but this line has {}.",
                    expected_indent,
                    if expected_indent == 1 { "" } else { "s" },
                    actual_indent
                )),
                data: Some(crate::rules::IssueData::WrongBodyIndent { expected_indent }),
            });
        }
    }

    // Recurse into subtask children
    for child in &subtask.children {
        issues.extend(check_subtask_body(child));
    }

    issues
}

#[cfg(test)]
mod tests;
