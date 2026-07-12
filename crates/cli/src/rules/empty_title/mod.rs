//! E016 — flags a task/subtask that has no title text left after the status
//! box (and any markers) are stripped, e.g. `- [ ] ` or a line consisting
//! only of markers such as `- [ ] #urgent`.

use crate::parser::{FileItem, Location, ParsingIssue, Subtask, TASK_LINE_PREFIX_LEN};
use crate::rules::Issue;

pub fn empty_title(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for item in items {
        if let FileItem::Task(task) = item {
            if task.parsing_issues.contains(&ParsingIssue::EmptyTitle) {
                issues.push(make_issue(&task.location, task.indent));
            }
            for subtask in &task.children {
                check_subtask_recursive(subtask, &mut issues);
            }
        }
    }

    issues
}

fn make_issue(location: &Location, indent: usize) -> Issue {
    Issue {
        location: location.clone(),
        code: crate::rules::ErrorCode::EmptyTitle,
        message: "Task has no title".to_string(),
        column: indent + TASK_LINE_PREFIX_LEN + 1,
        help: Some("Add a description after the status box.".to_string()),
        data: None,
    }
}

fn check_subtask_recursive(subtask: &Subtask, issues: &mut Vec<Issue>) {
    if subtask.parsing_issues.contains(&ParsingIssue::EmptyTitle) {
        issues.push(make_issue(&subtask.location, subtask.indent));
    }
    for child in &subtask.children {
        check_subtask_recursive(child, issues);
    }
}

#[cfg(test)]
mod tests;
