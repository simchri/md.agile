use crate::parser::{FileItem, Location, Subtask};
use crate::rules::Issue;

pub fn invalid_box(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for item in items {
        if let FileItem::Task(task) = item {
            if !task.box_valid {
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
        code: crate::rules::ErrorCode::BoxStyleInvalid,
        message: "Box style invalid".to_string(),
        column: indent + 1,
        help: Some("Valid task boxes look like this: [ ] [x] [-]".to_string()),
        data: None,
    }
}

fn check_subtask_recursive(subtask: &Subtask, issues: &mut Vec<Issue>) {
    if !subtask.box_valid {
        issues.push(make_issue(&subtask.location, subtask.indent));
    }
    for child in &subtask.children {
        check_subtask_recursive(child, issues);
    }
}

#[cfg(test)]
mod tests;
