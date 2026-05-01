//! E002 — flags items whose indentation does not match a valid subtask level.

use crate::parser::{FileItem, Subtask};
use crate::rules::Issue;

/// Flags wrong-indentation issues:
/// - Subtasks where `indent != depth * 2`.
/// - Top-level tasks with non-zero indentation that are *attached* to the
///   previous element (no preceding blank line). These were intended as
///   subtasks but got pushed to top-level by the parser due to bad spacing.
pub fn wrong_indentation(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for item in items {
        if let FileItem::Task(task) = item {
            // Top-level task with indent > 0 that was *attached* (not preceded
            // by a blank line) is wrong indentation, not an orphan.
            if task.indent > 0 && !task.preceded_by_blank {
                issues.push(Issue {
                    location: task.location.clone(),
                    code: "E002".to_string(),
                    message: "Wrong Indentation".to_string(),
                    column: task.indent + 1,
                    help: Some(format!(
                        "Indentation does not match a valid subtask level. Got {} space{}.",
                        task.indent,
                        if task.indent == 1 { "" } else { "s" }
                    )),
                });
            }

            // Recurse into subtasks.
            for subtask in &task.children {
                issues = check_subtask_recursive(subtask, 1, issues);
            }
        }
    }

    issues
}

/// Recursively walks a subtask tree, flagging any subtask whose indentation
/// does not match its expected nesting depth (`depth * 2`).
fn check_subtask_recursive(
    subtask: &Subtask,
    depth: usize,
    mut issues: Vec<Issue>,
) -> Vec<Issue> {
    let expected_indent = depth * 2;
    if subtask.indent != expected_indent {
        issues.push(Issue {
            location: subtask.location.clone(),
            code: "E002".to_string(),
            message: "Wrong Indentation".to_string(),
            column: subtask.indent + 1,
            help: Some(format!(
                "Expected {} space{} for depth {}, but got {}.",
                expected_indent,
                if expected_indent == 1 { "" } else { "s" },
                depth,
                subtask.indent
            )),
        });
    }

    for child in &subtask.children {
        issues = check_subtask_recursive(child, depth + 1, issues);
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use std::path::PathBuf;

    fn p(input: &str) -> Vec<FileItem> {
        parse(input, PathBuf::from("test.agile.md"))
    }

    #[test]
    fn flags_subtask_with_mismatched_indent() {
        // Subtask with 3 spaces instead of 2 (depth 1).
        let input = "\
- [ ] top
   - [ ] sub with 3 spaces instead of 2
";
        let issues = wrong_indentation(&p(input));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "E002");
        assert_eq!(issues[0].message, "Wrong Indentation");
    }

    #[test]
    fn passes_correctly_indented() {
        let input = "\
- [ ] top
  - [ ] depth 1
    - [ ] depth 2
      - [ ] depth 3
";
        let issues = wrong_indentation(&p(input));
        assert!(issues.is_empty());
    }
}
