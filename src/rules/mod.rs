//! Lint rules over a parsed `Vec<FileItem>`.
//!
//! Each rule is a free function `fn(&[FileItem]) -> Vec<Issue>` so the checker
//! can call all rules with the same shape and concatenate the results. Issues
//! carry a [`Location`] (file path + line number) so `agile check` can print
//! them in ESLint-style form.

use crate::parser::{FileItem, Location};

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

/// Flags indentation that is not a multiple of 2 (1, 3, 5... spaces).
///
/// Proper indentation uses 2 spaces per nesting level. Odd indentation is a
/// typo and should be fixed.
pub fn incorrect_indent(items: &[FileItem]) -> Vec<Issue> {
    items
        .iter()
        .filter_map(|item| match item {
            FileItem::Task(t) if t.indent > 0 && t.indent % 2 != 0 => Some(Issue {
                location: t.location.clone(),
                code:     "E002".to_string(),
                message:  "Incorrect Indentation".to_string(),
                column:   t.indent + 1, // 1-based column where the dash starts
                help:     Some(
                    format!("Use even spaces for indentation. This line has {} space{}; use {} or {}.",
                        t.indent,
                        if t.indent == 1 { "" } else { "s" },
                        if t.indent > 0 { t.indent - 1 } else { 0 },
                        t.indent + 1
                    )
                ),
            }),
            _ => None,
        })
        .collect()
}

/// Flags top-level tasks that are indented with even spacing but have no parent.
///
/// A task that ends up at the top level despite having leading whitespace is
/// almost always a typo: the user wrote it as a subtask, but a preceding blank
/// line broke the parent-child connection so the parser had nowhere to attach
/// it. We surface that as an issue.
pub fn wrong_indent(items: &[FileItem]) -> Vec<Issue> {
    items
        .iter()
        .filter_map(|item| match item {
            // Only flag if indent is even (0, 2, 4...) to avoid duplicate reporting
            // with incorrect_indent rule
            FileItem::Task(t) if t.indent > 0 && t.indent % 2 == 0 => Some(Issue {
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
mod tests {
    use super::*;
    use crate::parser::parse;
    use std::path::PathBuf;

    fn p(input: &str) -> Vec<FileItem> {
        parse(input, PathBuf::from("test.agile.md"))
    }

    #[test]
    fn wrong_indent_flags_orphaned_indented_task() {
        let input = "\
- [ ] real top level

  - [ ] orphan indented
";
        let issues = wrong_indent(&p(input));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].location.line, 3);
        assert_eq!(issues[0].code, "E001");
        assert!(issues[0].message.contains("Orphaned"));
    }

    #[test]
    fn wrong_indent_passes_clean_file() {
        let input = "\
- [ ] top
  - [ ] proper sub
- [x] another top
";
        let issues = wrong_indent(&p(input));
        assert!(issues.is_empty());
    }

    #[test]
    fn wrong_indent_flags_multiple() {
        let input = "\
- [ ] top one

  - [ ] orphan a

- [ ] top two

    - [ ] orphan b
";
        let issues = wrong_indent(&p(input));
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].location.line, 3);
        assert_eq!(issues[1].location.line, 7);
        assert_eq!(issues[0].column, 3); // 2 leading spaces + 1 for 1-based indexing
        assert_eq!(issues[1].column, 5); // 4 leading spaces + 1 for 1-based indexing
    }

    #[test]
    fn incorrect_indent_flags_odd_spaces() {
        let input = "\
- [ ] real top level

 - [ ] orphan with one space
";
        let issues = incorrect_indent(&p(input));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].location.line, 3);
        assert_eq!(issues[0].code, "E002");
        assert_eq!(issues[0].message, "Incorrect Indentation");
    }

    #[test]
    fn incorrect_indent_detects_multiple_odd_spaces() {
        let input = "\
- [ ] top one

   - [ ] orphan with 3 spaces

- [ ] top two

     - [ ] orphan with 5 spaces
";
        let issues = incorrect_indent(&p(input));
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].location.line, 3);
        assert_eq!(issues[0].column, 4); // 3 spaces + 1
        assert_eq!(issues[1].location.line, 7);
        assert_eq!(issues[1].column, 6); // 5 spaces + 1
    }

    #[test]
    fn incorrect_indent_passes_even_indentation() {
        let input = "\
- [ ] top

  - [ ] proper two spaces

    - [ ] proper four spaces
";
        let issues = incorrect_indent(&p(input));
        assert!(issues.is_empty());
    }
}
