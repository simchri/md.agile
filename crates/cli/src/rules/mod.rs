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
mod missing_required_subtasks;
mod missing_space_after_box;
mod orphaned_subtask;
mod unauthorized_completion;
mod undefined_assignment;
mod undefined_property;
mod unrequired_quoted_subtask;
mod uppercase_x;
mod wrong_body_indent;
mod wrong_indentation;

pub use incomplete_parent::incomplete_parent;
pub use invalid_box::invalid_box;
pub use missing_required_subtasks::missing_required_subtasks;
pub use missing_space_after_box::missing_space_after_box;
pub use orphaned_subtask::orphaned_subtask;
pub use unauthorized_completion::unauthorized_completion;
pub use undefined_assignment::undefined_assignment;
pub use undefined_property::undefined_property;
pub use unrequired_quoted_subtask::unrequired_quoted_subtask;
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
    /// E008: Property marker not declared in mdagile.toml
    UndefinedProperty,
    /// E009: Assignment marker not declared in mdagile.toml
    UndefinedAssignment,
    /// E010: Task is missing required subtasks mandated by a property
    MissingRequiredSubtasks,
    /// E011: Subtask uses the quoted syntax but is not declared as required by any property
    UnrequiredQuotedSubtask,
    /// E012: Required subtask was cancelled but the property doesn't allow cancellation
    CancelledRequiredSubtaskNotAllowed,
    /// E013: Task marked done by someone not authorized (not an assignee, nor a member of an assigned group)
    UnauthorizedCompletion,
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
            ErrorCode::UndefinedProperty => "E008",
            ErrorCode::UndefinedAssignment => "E009",
            ErrorCode::MissingRequiredSubtasks => "E010",
            ErrorCode::UnrequiredQuotedSubtask => "E011",
            ErrorCode::CancelledRequiredSubtaskNotAllowed => "E012",
            ErrorCode::UnauthorizedCompletion => "E013",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ErrorCode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "E001" => ErrorCode::OrphanedSubtask,
            "E002" => ErrorCode::WrongIndentation,
            "E003" => ErrorCode::WrongBodyIndentation,
            "E004" => ErrorCode::IncompleteParent,
            "E005" => ErrorCode::MissingSpaceAfterBox,
            "E006" => ErrorCode::BoxStyleInvalid,
            "E007" => ErrorCode::UppercaseX,
            "E008" => ErrorCode::UndefinedProperty,
            "E009" => ErrorCode::UndefinedAssignment,
            "E010" => ErrorCode::MissingRequiredSubtasks,
            "E011" => ErrorCode::UnrequiredQuotedSubtask,
            "E012" => ErrorCode::CancelledRequiredSubtaskNotAllowed,
            "E013" => ErrorCode::UnauthorizedCompletion,
            _ => return Err(()),
        })
    }
}

use crate::config::Config;
use crate::parser::{FileItem, Location, Marker, Subtask};
use serde::{Deserialize, Serialize};

/// Walks every task and subtask in `items`, calling `f` with the markers,
/// location, and indent of each node. Eliminates the boilerplate tree-walk
/// duplicated across rules.
pub fn for_each_node<F>(items: &[FileItem], mut f: F)
where
    F: FnMut(&[Marker], &Location, usize),
{
    for item in items {
        if let FileItem::Task(task) = item {
            f(&task.markers, &task.location, task.indent);
            walk_subtask_nodes(&task.children, &mut f);
        }
    }
}

fn walk_subtask_nodes<F>(subtasks: &[Subtask], f: &mut F)
where
    F: FnMut(&[Marker], &Location, usize),
{
    for sub in subtasks {
        f(&sub.markers, &sub.location, sub.indent);
        walk_subtask_nodes(&sub.children, f);
    }
}

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
    /// Payload for E008 "Undefined Property": the property name that is undefined.
    UndefinedProperty { property_name: String },
    /// Payload for E009 "Undefined Assignment": the assignment name that is undefined.
    UndefinedAssignment { assignment_name: String },
    /// Payload for E010 "Missing Required Subtasks": list of required subtask titles absent from the task.
    MissingRequiredSubtasks { missing: Vec<String> },
    /// Payload for E011 "Unrequired Quoted Subtask": the raw title of the incorrectly-quoted subtask.
    UnrequiredQuotedSubtask { title: String },
    /// Payload for E012 "Cancelled Required Subtask Not Allowed": the raw title of the
    /// required subtask that was cancelled without permission.
    CancelledRequiredSubtaskNotAllowed { title: String },
    /// Payload for E013 "Unauthorized Completion": the sorted list of user/group
    /// names that were authorized to complete this task.
    UnauthorizedCompletion { authorized: Vec<String> },
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
pub fn check_all(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    issues.extend(orphaned_subtask(items));
    issues.extend(wrong_indentation(items));
    issues.extend(wrong_body_indent(items));
    issues.extend(incomplete_parent(items));
    issues.extend(missing_space_after_box(items));
    issues.extend(invalid_box(items));
    issues.extend(uppercase_x(items));
    issues.extend(undefined_property(items, config));
    issues.extend(undefined_assignment(items, config));
    issues.extend(missing_required_subtasks(items, config));
    issues.extend(unrequired_quoted_subtask(items, config));
    issues
}

#[cfg(test)]
mod tests;
