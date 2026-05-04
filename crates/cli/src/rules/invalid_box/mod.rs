use crate::parser::{FileItem, Subtask, Task};
use crate::rules::Issue;

pub fn invalid_box(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for item in items {
        if let FileItem::Task(task) = item {
            if !task.box_valid {
                issues.push(box_not_valid_issue(task));
            }

            // Recurse into subtasks.
            for subtask in &task.children {
                issues = check_subtask_recursive(subtask, 1, issues);
            }
        }
    }

    issues
}

fn box_not_valid_issue(task: &Task) -> Issue {
    Issue {
        location: task.location.clone(),
        code: crate::rules::ErrorCode::BoxStyleInvalid,
        message: "Box style invalid".to_string(),
        column: task.indent + 1,
        help: Some(format!("Valid task boxes look like this: [ ] [x] [-]")),
        data: None,
    }
}

fn box_not_valid_issue_sub(task: &Subtask) -> Issue {
    Issue {
        location: task.location.clone(),
        code: crate::rules::ErrorCode::BoxStyleInvalid,
        message: "Box style invalid".to_string(),
        column: task.indent + 1,
        help: Some(format!("Valid task boxes look like this: [ ] [x] [-]")),
        data: None,
    }
}

 fn check_subtask_recursive(subtask: &Subtask, depth: usize, mut issues: Vec<Issue>) -> Vec<Issue> {
     if !subtask.box_valid {
         issues.push(box_not_valid_issue_sub(subtask));
     }

     for child in &subtask.children {
         issues = check_subtask_recursive(child, depth + 1, issues);
     }

     issues
 }

#[cfg(test)]
mod tests;
