//! Lint rules over a parsed `Vec<FileItem>`.
//!
//! Each rule is a free function `fn(&[FileItem]) -> Vec<Issue>` so the checker
//! can call all rules with the same shape and concatenate the results. Issues
//! carry a [`Location`] (file path + line number) so `agile check` can print
//! them in ESLint-style form.
//!
//! Each rule lives in its own submodule and is re-exported from this module
//! for convenience.

mod empty_title;
mod incomplete_parent;
mod invalid_box;
mod invalid_order;
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

pub use empty_title::empty_title;
pub use incomplete_parent::incomplete_parent;
pub use invalid_box::invalid_box;
pub use invalid_order::invalid_order;
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
    /// E014: Two ordered subtasks share the same order number among siblings
    DuplicateOrderNumber,
    /// E015: An ordered subtask was marked done while a lower-ordered sibling is still incomplete
    OutOfOrderCompletion,
    /// E016: Task/subtask has no title text after the status box (and markers, if any)
    EmptyTitle,
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
            ErrorCode::DuplicateOrderNumber => "E014",
            ErrorCode::OutOfOrderCompletion => "E015",
            ErrorCode::EmptyTitle => "E016",
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
            "E014" => ErrorCode::DuplicateOrderNumber,
            "E015" => ErrorCode::OutOfOrderCompletion,
            "E016" => ErrorCode::EmptyTitle,
            _ => return Err(()),
        })
    }
}

use crate::config::Config;
use crate::parser::{FileItem, Location, Marker, ParsingIssue, Status, Subtask, Task};
use serde::{Deserialize, Serialize};

/// A read-only view over a [`Task`] or [`Subtask`] that exposes the fields
/// rules typically need, so tree-walking rules can share one traversal
/// instead of each hand-rolling a parallel `check_task`/`check_subtask_recursive`
/// pair over the two (structurally near-identical) node types.
#[derive(Clone, Copy)]
pub enum NodeRef<'a> {
    Task(&'a Task),
    Subtask(&'a Subtask),
}

impl<'a> NodeRef<'a> {
    pub fn location(&self) -> &'a Location {
        match self {
            NodeRef::Task(t) => &t.location,
            NodeRef::Subtask(s) => &s.location,
        }
    }

    pub fn indent(&self) -> usize {
        match self {
            NodeRef::Task(t) => t.indent,
            NodeRef::Subtask(s) => s.indent,
        }
    }

    pub fn status(&self) -> &'a Status {
        match self {
            NodeRef::Task(t) => &t.status,
            NodeRef::Subtask(s) => &s.status,
        }
    }

    pub fn markers(&self) -> &'a [Marker] {
        match self {
            NodeRef::Task(t) => &t.markers,
            NodeRef::Subtask(s) => &s.markers,
        }
    }

    pub fn body(&self) -> &'a [String] {
        match self {
            NodeRef::Task(t) => &t.body,
            NodeRef::Subtask(s) => &s.body,
        }
    }

    pub fn children(&self) -> &'a [Subtask] {
        match self {
            NodeRef::Task(t) => &t.children,
            NodeRef::Subtask(s) => &s.children,
        }
    }

    pub fn parsing_issues(&self) -> &'a [ParsingIssue] {
        match self {
            NodeRef::Task(t) => &t.parsing_issues,
            NodeRef::Subtask(s) => &s.parsing_issues,
        }
    }

    pub fn title(&self) -> &'a str {
        match self {
            NodeRef::Task(t) => &t.title,
            NodeRef::Subtask(s) => &s.title,
        }
    }
}

/// Walks every task and subtask in `items`, calling `f` with a [`NodeRef`]
/// for each node. Eliminates the boilerplate recursive tree-walk (task +
/// mirrored `Subtask` recursion) duplicated across rules.
pub fn for_each_node<'a, F>(items: &'a [FileItem], mut f: F)
where
    F: FnMut(NodeRef<'a>),
{
    for item in items {
        if let FileItem::Task(task) = item {
            f(NodeRef::Task(task));
            walk_subtask_nodes(&task.children, &mut f);
        }
    }
}

fn walk_subtask_nodes<'a, F>(subtasks: &'a [Subtask], f: &mut F)
where
    F: FnMut(NodeRef<'a>),
{
    for sub in subtasks {
        f(NodeRef::Subtask(sub));
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

/// The result of resolving "who is completing this task" for the E013
/// "assignment / completion validation" check.
///
/// Distinct from a plain `Option<String>` because it must distinguish two
/// different reasons a check might not find a match:
/// - no identity could be determined at all (not a git repo, or `git config
///   user.email`/`user.name` both empty, and no `--as` override) — the
///   caller should silently skip the whole check in this case.
/// - an identity *was* determined (from git config or `--as`), but it
///   doesn't match any `[Users.X]` entry — this is always unauthorized for
///   any assigned task (it is never silently skipped).
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedIdentity {
    /// Git identity resolved to a known `[Users.X]` config key.
    Known(String),
    /// An identity was determined, but doesn't match any configured user.
    Unrecognized,
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

/// Runs every lint rule that doesn't need `Config` at all — i.e. everything
/// except [`undefined_property`], [`undefined_assignment`],
/// [`missing_required_subtasks`] and [`unrequired_quoted_subtask`], which all
/// look up property/user/group declarations.
///
/// Used when there's no config to trust yet (e.g. the LSP falls back to this
/// when `mdagile.toml` failed to load): running the config-dependent checks
/// against an empty placeholder `Config` would report every `#marker`/
/// `@marker` as undefined, which is spurious noise, not a real finding.
pub fn check_config_independent(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    issues.extend(orphaned_subtask(items));
    issues.extend(wrong_indentation(items));
    issues.extend(wrong_body_indent(items));
    issues.extend(incomplete_parent(items));
    issues.extend(missing_space_after_box(items));
    issues.extend(invalid_box(items));
    issues.extend(uppercase_x(items));
    issues.extend(invalid_order(items));
    issues.extend(empty_title(items));
    issues
}

/// Runs all lint rules and returns a concatenated list of issues.
pub fn check_all(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = check_config_independent(items);
    issues.extend(undefined_property(items, config));
    issues.extend(undefined_assignment(items, config));
    issues.extend(missing_required_subtasks(items, config));
    issues.extend(unrequired_quoted_subtask(items, config));
    issues
}

/// Checks whether marking `node` done right now would violate any
/// completion-related rule: "incomplete children" (E004), "missing required
/// subtasks" (E010), or "cancelled required subtask not allowed" (E012).
///
/// Used by `agile task done <address>` to validate a single addressed node
/// in isolation, without running the full rule set over the rest of the
/// project (which `task done` explicitly avoids for efficiency — see
/// `doc/cli-structure.md`). Reuses the exact same rule logic as `agile
/// check` so the two commands never disagree about what counts as a valid
/// completion.
pub fn check_completable(node: NodeRef, config: &Config) -> Vec<Issue> {
    let mut issues =
        incomplete_parent::check_children_complete(node.children(), node.location(), node.indent());
    issues.extend(missing_required_subtasks::check_node(
        node.markers(),
        node.children(),
        node.location(),
        node.indent(),
        config,
    ));
    issues
}

/// Returns whether `identity` is eligible to work on `node`: `true` if the
/// node carries no `@user`/`@group` assignment markers at all (unassigned
/// tasks are open to anyone, mirroring the E013 `unauthorized_completion`
/// philosophy that assignment never restricts *unassigned* tasks), or if
/// `identity` is directly assigned or a member of an assigned group.
///
/// Used by `agile task next --mine`.
pub fn is_eligible_for(node: NodeRef, identity: &ResolvedIdentity, config: &Config) -> bool {
    let names = unauthorized_completion::assignment_names(node.markers());
    if names.is_empty() {
        return true;
    }
    let authorized = unauthorized_completion::authorized_users(&names, config);
    match identity {
        ResolvedIdentity::Known(user) => authorized.iter().any(|a| a == user),
        ResolvedIdentity::Unrecognized => false,
    }
}

#[cfg(test)]
mod tests;
