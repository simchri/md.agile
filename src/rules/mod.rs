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
mod tests;
