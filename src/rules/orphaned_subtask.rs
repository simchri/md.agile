//! E001 — flags top-level tasks that look like subtasks but have no parent.

use crate::parser::FileItem;
use crate::rules::Issue;

/// Flags top-level tasks that look like indented subtasks but are separated
/// from the previous element by a blank line (so they have no parent).
///
/// A task is "orphaned" when:
/// - It has non-zero indentation, AND
/// - It is preceded by a blank line (or appears at file start).
///
/// If a top-level task with non-zero indentation is *not* preceded by a blank
/// line, it was attached to a previous element but became top-level due to
/// wrong indentation — that is reported by `wrong_indentation` (E002) instead.
pub fn orphaned_subtask(items: &[FileItem]) -> Vec<Issue> {
    items
        .iter()
        .filter_map(|item| match item {
            FileItem::Task(t) if t.indent > 0 && t.preceded_by_blank => Some(Issue {
                location: t.location.clone(),
                code:     "E001".to_string(),
                message:  "Orphaned Subtask".to_string(),
                column:   t.indent + 1, // 1-based column where the dash starts
                help:     Some(
                    "Remove leading spaces (make this a task), or delete preceeding empty lines if the element above is a task (make this a subtask)."
                        .to_string()
                ),
            }),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
#[path = "orphaned_subtask_tests.rs"]
mod tests;
