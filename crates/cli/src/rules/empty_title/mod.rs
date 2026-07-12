//! E016 — flags a task/subtask that has no title text left after the status
//! box (and any markers) are stripped, e.g. `- [ ] ` or a line consisting
//! only of markers such as `- [ ] #urgent`.

use crate::parser::{FileItem, Location, ParsingIssue, TASK_LINE_PREFIX_LEN};
use crate::rules::{Issue, for_each_node};

pub fn empty_title(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        if node.parsing_issues().contains(&ParsingIssue::EmptyTitle) {
            issues.push(make_issue(node.location(), node.indent()));
        }
    });
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

#[cfg(test)]
mod tests;
