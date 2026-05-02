//! E004 — flags done parents with incomplete non-optional children.

use crate::parser::{FileItem, SpecialMarker, Status, Subtask};
use crate::rules::Issue;

/// Flags tasks/subtasks marked done `[x]` that have incomplete children.
/// A done parent with incomplete children is invalid unless the children are optional (`#OPT`).
/// Cancelled parents are ignored.
pub fn incomplete_parent(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for item in items {
        if let FileItem::Task(task) = item {
            issues.extend(check_task_completion(task));
        }
    }

    issues
}

fn check_task_completion(task: &crate::parser::Task) -> Vec<Issue> {
    let mut issues = Vec::new();

    // Check task's children
    if task.status == Status::Done {
        issues.extend(check_children_complete(&task.children, &task.location));
    }

    // Recurse into subtasks
    for subtask in &task.children {
        issues.extend(check_subtask_completion(subtask));
    }

    issues
}

fn check_subtask_completion(subtask: &Subtask) -> Vec<Issue> {
    let mut issues = Vec::new();

    // Check if this subtask is done but has incomplete children
    if subtask.status == Status::Done {
        issues.extend(check_children_complete(&subtask.children, &subtask.location));
    }

    // Recurse into subtask children
    for child in &subtask.children {
        issues.extend(check_subtask_completion(child));
    }

    issues
}

fn check_children_complete(children: &[Subtask], parent_location: &crate::parser::Location) -> Vec<Issue> {
    let mut issues = Vec::new();

    for child in children {
        // Skip checking optional children - they don't block parent completion
        let is_optional = child.markers.iter().any(|m| {
            matches!(m, crate::parser::Marker::Special(SpecialMarker::Opt))
        });

        if is_optional {
            continue;
        }

        // If child is not done and not cancelled, parent completion is invalid
        if child.status != Status::Done && child.status != Status::Cancelled {
            issues.push(Issue {
                location: parent_location.clone(),
                code: crate::rules::ErrorCode::IncompleteParent,
                message: "Incomplete parent".to_string(),
                column: 1,
                help: Some(
                    "This task is marked done, but it has incomplete children. \
                     Mark all required children done, cancel them, or make them optional with #OPT."
                        .to_string()
                ),
                data: None,
            });
            break; // Only report once per parent
        }
    }

    issues
}

#[cfg(test)]
mod tests;
