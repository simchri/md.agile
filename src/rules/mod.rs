//! Lint rules over a parsed `Vec<FileItem>`.
//!
//! Each rule is a free function `fn(&[FileItem]) -> Vec<Issue>` so the checker
//! can call all rules with the same shape and concatenate the results. Issues
//! carry a [`Location`] (file path + line number) so `agile check` can print
//! them in ESLint-style form.
//!
//! Each rule lives in its own submodule and is re-exported from this module
//! for convenience.

mod orphaned_subtask;
mod wrong_indentation;

pub use orphaned_subtask::orphaned_subtask;
pub use wrong_indentation::wrong_indentation;

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

/// Runs all lint rules and returns a concatenated list of issues.
pub fn check_all(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    issues.extend(orphaned_subtask(items));
    issues.extend(wrong_indentation(items));
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
    fn wrong_indentation_vs_orphan_distinction_1() {
        // An Orphan task is:
        // - task that is separated by an empty line from previous elements
        // - AND has any indentation other than zero.
        //
        // Wrong indentation
        // - a sub task / task that is attached to a parent task (no empty line between other element)
        // - AND that has an indentation that does not match the subtask level

        let input = "\
- [ ] testing
  - [ ] subtask -> OK
  - [ ] another subtask -> OK
   - [ ] subtask WRONG INDENT
 - [ ] subtask WRONG INDENT

 - [ ] ORPHAN (also incorrect indent, but message should be orphan)

  - [ ] ORPHAN
";

        let mut issues = check_all(&p(input));
        issues.sort_by_key(|i| i.location.line);

        assert_eq!(issues[0].code, "E002");
        assert_eq!(issues[1].code, "E002");
        assert_eq!(issues[2].code, "E001");
        assert_eq!(issues[3].code, "E001");
    }
}
