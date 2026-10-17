//! Lint rules over a parsed `Vec<FileItem>`.
//!
//! Each rule is a free function `fn(&[FileItem]) -> Vec<Issue>` so the checker
//! can call all rules with the same shape and concatenate the results. Issues
//! carry a [`Location`] (file path + line number) so `agile check` can print
//! them in ESLint-style form.
//!
//! Each rule lives in its own submodule and is re-exported from this module
//! for convenience.

mod branch_properties;
mod colliding_field_labels;
mod empty_title;
mod field_left_unfilled;
mod field_value_not_allowed;
mod incomplete_parent;
mod invalid_box;
mod invalid_milestone;
mod invalid_order;
mod missing_required_field;
mod missing_required_subtasks;
mod missing_space_after_box;
mod missing_space_after_field_label;
mod multiline_field_filled_inline;
mod neighbor_task_on_top_level;
mod neighbor_tasks;
mod next_task;
mod orphaned_subtask;
mod unauthorized_completion;
mod undefined_assignment;
mod undefined_property;
mod unrequired_quoted_subtask;
mod uppercase_x;
mod wrong_body_indent;
mod wrong_indentation;

pub use branch_properties::{
    plain_branching_property, undefined_branch_outcome, unresolved_branch_property,
};
pub use colliding_field_labels::colliding_field_labels;
pub use empty_title::empty_title;
pub use field_left_unfilled::field_left_unfilled;
pub use field_value_not_allowed::field_value_not_allowed;
pub use incomplete_parent::incomplete_parent;
pub use invalid_box::invalid_box;
pub use invalid_milestone::invalid_milestone;
pub use invalid_order::invalid_order;
pub use missing_required_field::missing_required_field;
pub use missing_required_subtasks::missing_required_subtasks;
pub use missing_space_after_box::missing_space_after_box;
pub use missing_space_after_field_label::missing_space_after_field_label;
pub use multiline_field_filled_inline::multiline_field_filled_inline;
pub use neighbor_task_on_top_level::neighbor_task_on_top_level;
pub use neighbor_tasks::neighbor_tasks;
pub use next_task::{
    find_next_actionable, has_closed_work, is_eligible_for, is_next_task, is_previous_task,
};
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
    /// E017: Two milestones (anywhere in the project) share the same name
    DuplicateMilestoneName,
    /// E018: A `#MILESTONE` header has no name
    MissingMilestoneName,
    /// E019: A property's configured `neighbortasks` entry isn't present as a
    /// sibling of the property-carrying task/subtask
    MissingNeighborTask,
    /// E020: A property requiring neighbor tasks was set on a top-level task,
    /// where it can never be satisfied
    NeighborTaskOnTopLevel,
    /// E021: A task was marked done while a Branch Property (see
    /// README.vision.md "Branch Properties") is still in its pending form
    /// (e.g. `#review:...`), with no outcome chosen yet
    UnresolvedBranchProperty,
    /// E022: A Branch Property was resolved (e.g. `#review:passed`) to a
    /// branch name that isn't declared in config
    UndefinedBranchOutcome,
    /// E023: A property that declares branches (see README.vision.md
    /// "Branch Properties") was used in its plain form (`#review`) instead
    /// of pending (`#review:...`) or resolved (`#review:passed`)
    PlainBranchingProperty,
    /// E024: A property attaching a `[Form.X]` (see README.vision.md
    /// "Forms & Fields") is missing one or more of that form's fields
    /// entirely -- no matching `__Label:__ value` line at all
    MissingRequiredField,
    /// E025: A field label line is missing the required space before its
    /// value, e.g. `__Email:__john@example.com` instead of
    /// `__Email:__ john@example.com`
    MissingSpaceAfterFieldLabel,
    /// E026: A present form field's value still equals its resolved
    /// "unfilled placeholder" (the field's configured `default`, or the
    /// generic `___` when it has none) -- i.e. the field line exists but
    /// nobody has actually filled it out yet.
    FieldLeftUnfilled,
    /// E027: Two distinct properties on the same task/subtask each attach a
    /// `[Form.X]` (see README.vision.md "Forms & Fields"), and those two
    /// forms declare a field with the same display label -- the one
    /// `__Label:__ value` body line could never unambiguously satisfy both
    /// forms at once.
    CollidingFieldLabels,
    /// E028: A field configured as multiline (see
    /// [`crate::config::FieldConfig::multiline`]) was instead written
    /// inline, i.e. using the single-line grammar (`__Label:__ value`)
    /// rather than opening the label on its own line with the content on
    /// the line(s) below it.
    MultilineFieldFilledInline,
    /// E029: A field configured with a closed set of `values` (see
    /// [`crate::config::FieldConfig::values`]) has an actual, filled-in
    /// value that isn't one of those allowed values.
    FieldValueNotAllowed,
    /// E030: A form's configured validation script ran and reported the
    /// form (or one of its fields) as invalid.
    FormValidationFailed,
    /// E031: A form's configured validation script could not be run
    /// properly (not trusted, missing, crashed, timed out, bad output).
    FormScriptFailed,
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
            ErrorCode::DuplicateMilestoneName => "E017",
            ErrorCode::MissingMilestoneName => "E018",
            ErrorCode::MissingNeighborTask => "E019",
            ErrorCode::NeighborTaskOnTopLevel => "E020",
            ErrorCode::UnresolvedBranchProperty => "E021",
            ErrorCode::UndefinedBranchOutcome => "E022",
            ErrorCode::PlainBranchingProperty => "E023",
            ErrorCode::MissingRequiredField => "E024",
            ErrorCode::MissingSpaceAfterFieldLabel => "E025",
            ErrorCode::FieldLeftUnfilled => "E026",
            ErrorCode::CollidingFieldLabels => "E027",
            ErrorCode::MultilineFieldFilledInline => "E028",
            ErrorCode::FieldValueNotAllowed => "E029",
            ErrorCode::FormValidationFailed => "E030",
            ErrorCode::FormScriptFailed => "E031",
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
            "E017" => ErrorCode::DuplicateMilestoneName,
            "E018" => ErrorCode::MissingMilestoneName,
            "E019" => ErrorCode::MissingNeighborTask,
            "E020" => ErrorCode::NeighborTaskOnTopLevel,
            "E021" => ErrorCode::UnresolvedBranchProperty,
            "E022" => ErrorCode::UndefinedBranchOutcome,
            "E023" => ErrorCode::PlainBranchingProperty,
            "E024" => ErrorCode::MissingRequiredField,
            "E025" => ErrorCode::MissingSpaceAfterFieldLabel,
            "E026" => ErrorCode::FieldLeftUnfilled,
            "E027" => ErrorCode::CollidingFieldLabels,
            "E028" => ErrorCode::MultilineFieldFilledInline,
            "E029" => ErrorCode::FieldValueNotAllowed,
            "E030" => ErrorCode::FormValidationFailed,
            "E031" => ErrorCode::FormScriptFailed,
            _ => return Err(()),
        })
    }
}

use crate::config::Config;
use crate::parser::{
    FieldLine, FileItem, Location, MalformedFieldLine, Marker, Order, ParsingIssue, Status,
    Subtask, Task,
};
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

    pub fn fields(&self) -> &'a [FieldLine] {
        match self {
            NodeRef::Task(t) => &t.fields,
            NodeRef::Subtask(s) => &s.fields,
        }
    }

    pub fn malformed_fields(&self) -> &'a [MalformedFieldLine] {
        match self {
            NodeRef::Task(t) => &t.malformed_fields,
            NodeRef::Subtask(s) => &s.malformed_fields,
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

    /// The node's own ordering, if any. Always `None` for a top-level
    /// [`Task`] — only [`Subtask`] carries an [`Order`].
    pub fn order(&self) -> Option<&'a Order> {
        match self {
            NodeRef::Task(_) => None,
            NodeRef::Subtask(s) => Some(&s.order),
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

/// Locates the (sub)task starting at the given 1-based source line, scanning
/// `items` in the same document order as [`for_each_node`] but returning as
/// soon as a match is found instead of visiting every node.
///
/// Used to resolve a (file, line) identity — e.g. one captured client-side by
/// the GUI from an earlier task listing — back to a concrete node, without
/// requiring the caller to know its position in the tree (task index / child
/// indices) the way [`crate::cli::subcommands::task::resolve_address`] does
/// for CLI addresses. Returns `None` if no task or subtask starts at that
/// line, e.g. because the file changed since the line was captured.
pub fn find_node_by_line(items: &[FileItem], line: usize) -> Option<NodeRef<'_>> {
    fn find_in_subtasks(subtasks: &[Subtask], line: usize) -> Option<NodeRef<'_>> {
        for sub in subtasks {
            if sub.location.line == line {
                return Some(NodeRef::Subtask(sub));
            }
            if let Some(found) = find_in_subtasks(&sub.children, line) {
                return Some(found);
            }
        }
        None
    }

    for item in items {
        if let FileItem::Task(task) = item {
            if task.location.line == line {
                return Some(NodeRef::Task(task));
            }
            if let Some(found) = find_in_subtasks(&task.children, line) {
                return Some(found);
            }
        }
    }
    None
}

/// Finds the sibling list (i.e. the parent's `children`) that contains the
/// subtask starting at `line`. Returns `None` if `line` doesn't identify a
/// subtask at all (e.g. it's a top-level task, which has no `Order` and no
/// meaningful "siblings" for ordering purposes) or doesn't exist.
///
/// Used by [`check_completable`] to validate ordered-subtask completion
/// (E015) *before* a node is marked done, the same way [`find_node_by_line`]
/// locates the node itself.
fn find_siblings_by_line(items: &[FileItem], line: usize) -> Option<&[Subtask]> {
    fn find_in(subtasks: &[Subtask], line: usize) -> Option<&[Subtask]> {
        for sub in subtasks {
            if sub.location.line == line {
                return Some(subtasks);
            }
            if let Some(found) = find_in(&sub.children, line) {
                return Some(found);
            }
        }
        None
    }

    for item in items {
        if let FileItem::Task(task) = item {
            if let Some(found) = find_in(&task.children, line) {
                return Some(found);
            }
        }
    }
    None
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
    /// Payload for E019 "Missing Neighbor Task": list of required neighbor
    /// task titles absent from the property-carrying node's siblings.
    MissingNeighborTask { missing: Vec<String> },
    /// Payload for E021 "Unresolved Branch Property": base names of every
    /// still-pending (`BranchPending`) branch property on the flagged node,
    /// in the order they appear in its markers.
    UnresolvedBranchProperty { properties: Vec<String> },
    /// Payload for E024 "Missing Required Field": labels of the form fields
    /// entirely absent from the property-carrying node's body.
    MissingRequiredField { missing: Vec<String> },
    /// Payload for E025 "Missing Space After Field Label": the field label
    /// that's missing its separating space before the value.
    MissingSpaceAfterFieldLabel { label: String },
    /// Payload for E026 "Field Left Unfilled": the label of the present
    /// form field whose value still equals its resolved unfilled
    /// placeholder.
    FieldLeftUnfilled { label: String },
    /// Payload for E027 "Colliding Field Labels": the shared display label,
    /// and the names of the two (or more) properties on the node whose
    /// attached forms each declare a field with that label.
    CollidingFieldLabels {
        label: String,
        properties: Vec<String>,
    },
    /// Payload for E028 "Multiline Field Filled Inline": the label of the
    /// multiline-configured field that was instead written inline.
    MultilineFieldFilledInline { label: String },
    /// Payload for E029 "Field Value Not Allowed": the field's label, its
    /// actual (disallowed) value, and the configured list of allowed
    /// values.
    FieldValueNotAllowed {
        label: String,
        value: String,
        allowed: Vec<String>,
    },
    /// Payload for E030 "Form Validation Failed": the form, the offending
    /// field label (if the script named one) and the script's message.
    FormValidationFailed {
        form: String,
        field: Option<String>,
        message: String,
    },
    /// Payload for E031 "Form Script Failed": the form and its script path.
    FormScriptFailed { form: String, script: String },
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
/// [`missing_required_subtasks`], [`missing_required_field`],
/// [`field_left_unfilled`], [`colliding_field_labels`],
/// [`multiline_field_filled_inline`], [`field_value_not_allowed`],
/// [`unrequired_quoted_subtask`], [`neighbor_task_on_top_level`],
/// [`neighbor_tasks`], [`undefined_branch_outcome`] and
/// [`plain_branching_property`], which all look up property/user/group
/// declarations.
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
    issues.extend(missing_space_after_field_label(items));
    issues.extend(invalid_box(items));
    issues.extend(uppercase_x(items));
    issues.extend(invalid_order(items));
    issues.extend(empty_title(items));
    issues.extend(invalid_milestone(items));
    issues.extend(unresolved_branch_property(items));
    issues
}

/// Runs all lint rules and returns a concatenated list of issues.
pub fn check_all(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = check_config_independent(items);
    issues.extend(undefined_property(items, config));
    issues.extend(undefined_assignment(items, config));
    issues.extend(undefined_branch_outcome(items, config));
    issues.extend(plain_branching_property(items, config));
    issues.extend(missing_required_subtasks(items, config));
    issues.extend(missing_required_field(items, config));
    issues.extend(colliding_field_labels(items, config));
    issues.extend(field_left_unfilled(items, config));
    issues.extend(multiline_field_filled_inline(items, config));
    issues.extend(field_value_not_allowed(items, config));
    issues.extend(unrequired_quoted_subtask(items, config));
    issues.extend(neighbor_task_on_top_level(items, config));
    issues.extend(neighbor_tasks(items, config));
    issues
}

/// Checks whether marking `node` done right now would violate any
/// completion-related rule: "incomplete children" (E004), "missing required
/// subtasks" (E010), "field left unfilled" (E026), "cancelled required
/// subtask not allowed" (E012), "unauthorized completion" (E013, against
/// `identity`), "unresolved branch property" (E021), "undefined branch
/// outcome" (E022), "plain branching property" (E023) — or, if `node` is an
/// ordered subtask — "out-of-order completion" (E015).
///
/// `items` is the full parsed file `node` came from; it's only used to
/// locate `node`'s siblings for the ordering check (see
/// [`find_siblings_by_line`]) and is otherwise unused here.
///
/// `identity` is the resolved acting identity (see
/// [`crate::checker::resolve_task_done_identity`]) used for the E013 check:
/// unlike the git-diff-based `agile check`, which only ever *detects* an
/// unauthorized completion after the fact, this rejects it *before* the
/// write happens — so `node` never needs to have already been flipped to
/// `[x]` for the check to apply.
///
/// Used by `agile task done <address>` (and the GUI's `mark_task_done`, via
/// [`crate::cli::subcommands::task::mark_node_done`]) to validate a single
/// addressed node in isolation, without running the full rule set over the
/// rest of the project (which `task done` explicitly avoids for efficiency —
/// see `doc/development/cli_structure.md`). Reuses the exact same rule logic as `agile
/// check` so the two commands never disagree about what counts as a valid
/// completion.
pub fn check_completable(
    items: &[FileItem],
    node: NodeRef,
    config: &Config,
    identity: &ResolvedIdentity,
) -> Vec<Issue> {
    let mut issues =
        incomplete_parent::check_children_complete(node.children(), node.location(), node.indent());
    issues.extend(missing_required_subtasks::check_node(
        node.markers(),
        node.children(),
        node.location(),
        node.indent(),
        config,
    ));
    issues.extend(field_left_unfilled::check_node(
        node.markers(),
        node.fields(),
        config,
    ));
    issues.extend(branch_properties::check_unresolved(
        node.markers(),
        node.location(),
        node.indent(),
    ));
    issues.extend(branch_properties::check_undefined_outcome(
        node.markers(),
        node.location(),
        node.indent(),
        config,
    ));
    issues.extend(branch_properties::check_plain_form(
        node.markers(),
        node.location(),
        node.indent(),
        config,
    ));
    issues.extend(check_order_completable(items, node));
    issues.extend(check_authorized_completable(node, config, identity));
    issues
}

/// The E013 half of [`check_completable`]: if `node` carries `@user`/`@group`
/// assignment markers, checks whether `identity` is authorized to complete
/// it — directly assigned, or a member of an assigned group (see
/// [`unauthorized_completion::authorized_users`]) — and, if not, refuses the
/// completion with the same "Unauthorized Completion" issue the git-diff-based
/// `agile check`/E013 rule ([`unauthorized_completion::unauthorized_completion`])
/// would otherwise only catch after the fact. Nodes with no assignment
/// markers at all are never flagged — unassigned tasks stay open to anyone,
/// matching E013's own philosophy.
fn check_authorized_completable(
    node: NodeRef,
    config: &Config,
    identity: &ResolvedIdentity,
) -> Vec<Issue> {
    let names = unauthorized_completion::assignment_names(node.markers());
    if names.is_empty() {
        return Vec::new();
    }
    let authorized = unauthorized_completion::authorized_users(&names, config);
    let is_authorized = unauthorized_completion::is_authorized_user(&authorized, identity);
    if is_authorized {
        return Vec::new();
    }
    vec![Issue {
        location: node.location().clone(),
        code: ErrorCode::UnauthorizedCompletion,
        message: format!(
            "Task marked done by an unauthorized user; only {} may complete it",
            authorized
                .iter()
                .map(|a| format!("\"{a}\""))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        column: node.indent() + 1,
        help: Some(
            "Only assigned users (or members of an assigned group) should mark this task done."
                .to_string(),
        ),
        data: Some(IssueData::UnauthorizedCompletion { authorized }),
    }]
}

/// The E015 half of [`check_completable`]: if `node` is an ordered subtask,
/// checks whether any lower-ordered sibling is still incomplete (not done,
/// not cancelled) and, if so, refuses the completion — pre-empting the
/// out-of-order state that [`invalid_order::invalid_order`] would otherwise
/// only catch after the fact, on the next `agile check`.
fn check_order_completable(items: &[FileItem], node: NodeRef) -> Vec<Issue> {
    let Some(Order::Ordered(order_number)) = node.order() else {
        return Vec::new();
    };
    let NodeRef::Subtask(sub) = node else {
        return Vec::new();
    };
    let Some(siblings) = find_siblings_by_line(items, node.location().line) else {
        return Vec::new();
    };
    if invalid_order::blocked_by_incomplete_lower_order(siblings, *order_number) {
        vec![Issue {
            location: node.location().clone(),
            code: ErrorCode::OutOfOrderCompletion,
            message: "Ordered task completed out of order".to_string(),
            column: invalid_order::order_number_column(sub),
            help: Some(
                "A lower-ordered sibling is still incomplete. Complete or cancel lower-ordered \
                 siblings first."
                    .to_string(),
            ),
            data: None,
        }]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
