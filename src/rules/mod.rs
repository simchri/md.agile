//! Lint rules over a parsed `Vec<FileItem>`.
//!
//! Each rule is a free function `fn(&[FileItem]) -> Vec<Issue>` so the checker
//! can call all rules with the same shape and concatenate the results. Issues
//! carry a [`Location`] (file path + line number) so `agile check` can print
//! them in ESLint-style form.

use crate::parser::{FileItem, Location, Subtask};

/// A single problem found by a rule.
///
/// `location` points at the source line that triggered the issue; `code` is a
/// machine-readable identifier (e.g., "E001"); `message` is the human-readable
/// description; `column` marks the character position (1-based) where the issue
/// occurs; `help` provides optional guidance on fixing the issue.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub location: Location,
    pub code: String,
    pub message: String,
    pub column: usize,
    pub help: Option<String>,
}

/// Flags top-level tasks with non-zero indentation.
///
/// A top-level task should have indent=0. If it has indent > 0, it means the
/// source line was indented but the parser couldn't find a parent (likely due to
/// a preceding blank line breaking the connection). These are "orphaned" subtasks.
pub fn orphaned_subtask(items: &[FileItem]) -> Vec<Issue> {
    items
        .iter()
        .filter_map(|item| match item {
            FileItem::Task(t) if t.indent > 0 => Some(Issue {
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

/// Flags tasks/subtasks with indentation that doesn't match their nesting depth.
///
/// Valid indentation is `depth * 2` spaces (0 for top-level, 2 for depth 1, 4 for depth 2, etc).
/// Any deviation signals either a typo in spacing or incorrect parsing context.
fn check_wrong_indent_recursive(
    subtask: &Subtask,
    depth: usize,
    mut issues: Vec<Issue>,
) -> Vec<Issue> {
    let expected_indent = depth * 2;
    if subtask.indent != expected_indent {
        issues.push(Issue {
            location: subtask.location.clone(),
            code:     "E002".to_string(),
            message:  "Wrong Indentation".to_string(),
            column:   subtask.indent + 1,
            help:     Some(
                format!(
                    "Expected {} space{} for depth {}, but got {}.",
                    expected_indent,
                    if expected_indent == 1 { "" } else { "s" },
                    depth,
                    subtask.indent
                )
            ),
        });
    }

    for child in &subtask.children {
        issues = check_wrong_indent_recursive(child, depth + 1, issues);
    }

    issues
}

pub fn wrong_indentation(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for item in items {
        if let FileItem::Task(task) = item {
            // Check all subtasks recursively (top-level tasks are checked by orphaned_subtask)
            for subtask in &task.children {
                issues = check_wrong_indent_recursive(subtask, 1, issues);
            }
        }
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
    fn orphaned_subtask_flags_task_with_indent() {
        let input = "\
- [ ] real top level

  - [ ] orphan indented
";
        let issues = orphaned_subtask(&p(input));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].location.line, 3);
        assert_eq!(issues[0].code, "E001");
        assert!(issues[0].message.contains("Orphaned"));
    }

    #[test]
    fn orphaned_subtask_passes_clean_file() {
        let input = "\
- [ ] top
  - [ ] proper sub
- [x] another top
";
        let issues = orphaned_subtask(&p(input));
        assert!(issues.is_empty());
    }

    #[test]
    fn orphaned_subtask_flags_multiple() {
        let input = "\
- [ ] top one

  - [ ] orphan a

- [ ] top two

    - [ ] orphan b
";
        let issues = orphaned_subtask(&p(input));
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].location.line, 3);
        assert_eq!(issues[1].location.line, 7);
    }

    #[test]
    fn wrong_indentation_flags_any_mismatched_indent() {
        // Create a subtask with wrong indent (3 spaces instead of 2)
        // The parser will parse it as depth 1 (3 / 2 = 1), but indent will be 3
        let input = "\
- [ ] top
   - [ ] sub with 3 spaces instead of 2
";
        let issues = wrong_indentation(&p(input));
        let wrong_indent_issues: Vec<_> = issues.iter().filter(|i| i.code == "E002").collect();
        assert_eq!(wrong_indent_issues.len(), 1);
        assert_eq!(wrong_indent_issues[0].message, "Wrong Indentation");
    }

    #[test]
    fn wrong_indentation_passes_correctly_indented() {
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
