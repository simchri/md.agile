//! Lint rules over a parsed `Vec<FileItem>`.
//!
//! Each rule is a free function `fn(&[FileItem]) -> Vec<Issue>` so the checker
//! can call all rules with the same shape and concatenate the results. Issues
//! carry a [`Location`] (file path + line number) so `agile check` can print
//! them in ESLint-style form.
//!
//! Each rule lives in its own submodule and is re-exported from this module
//! for convenience.

mod incomplete_parent;
mod invalid_box;
mod missing_space_after_box;
mod orphaned_subtask;
mod uppercase_x;
mod wrong_body_indent;
mod wrong_indentation;

pub use incomplete_parent::incomplete_parent;
pub use invalid_box::invalid_box;
pub use missing_space_after_box::missing_space_after_box;
pub use orphaned_subtask::orphaned_subtask;
pub use uppercase_x::uppercase_x;
pub use wrong_body_indent::wrong_body_indent;
pub use wrong_indentation::wrong_indentation;

/// Error codes for validation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// E001: Orphaned indented task with no parent
    OrphanedSubtask,
    /// E002: Task indentation doesn't match nesting depth
    WrongIndentation,
    /// E003: Task body indentation misaligned
    WrongBodyIndentation,
    /// E004: Done parent with incomplete children
    IncompleteParent,
    /// E005: Missing space after status box
    MissingSpaceAfterBox,
    /// E006: Box style invalid
    BoxStyleInvalid,
    /// E007: Uppercase X used instead of lowercase x
    UppercaseX,
}

impl ErrorCode {
    /// Returns the short code string (e.g., "E001")
    pub fn as_str(&self) -> &'static str {
        match self {
            // SFI: Logically organized error codes
            ErrorCode::OrphanedSubtask => "E001",
            ErrorCode::WrongIndentation => "E002",
            ErrorCode::WrongBodyIndentation => "E003",
            ErrorCode::IncompleteParent => "E004",
            ErrorCode::MissingSpaceAfterBox => "E005",
            ErrorCode::BoxStyleInvalid => "E006",
            ErrorCode::UppercaseX => "E007",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

use crate::parser::{FileItem, Location};
use serde::{Deserialize, Serialize};

/// Machine-readable, rule-specific payload attached to an [`Issue`].
///
/// Lets consumers (notably the LSP code-action handler) act on the issue
/// without re-deriving information that the rule already computed. Variants
/// are added per rule as needed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IssueData {
    /// Payload for E002 "Wrong Indentation": the indentation (in spaces)
    /// that the offending line should be re-indented to.
    WrongIndent { expected_indent: usize },
    /// Payload for E003 "Wrong Body Indentation": the indentation (in spaces)
    /// that the body line should be re-indented to.
    WrongBodyIndent { expected_indent: usize },
    /// Payload for E005 "Missing Space After Box": marker to add space after status box.
    MissingSpaceAfterBox,
}

/// A single problem found by a rule.
///
/// `location` points at the source line that triggered the issue; `code` is a
/// machine-readable error code (e.g., E001); `message` is the human-readable
/// description; `column` marks the character position (1-based) where the issue
/// occurs; `help` provides optional guidance on fixing the issue; `data`
/// carries an optional rule-specific payload (used e.g. by the LSP layer to
/// build code-action edits).
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub location: Location,
    pub code: ErrorCode,
    pub message: String,
    pub column: usize,
    pub help: Option<String>,
    pub data: Option<IssueData>,
}

/// Runs all lint rules and returns a concatenated list of issues.
pub fn check_all(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    issues.extend(orphaned_subtask(items));
    issues.extend(wrong_indentation(items));
    issues.extend(wrong_body_indent(items));
    issues.extend(incomplete_parent(items));
    issues.extend(missing_space_after_box(items));
    issues.extend(invalid_box(items));
    issues.extend(uppercase_x(items));
    issues
}

#[cfg(test)]
mod tests;
