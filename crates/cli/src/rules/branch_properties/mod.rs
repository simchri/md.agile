//! E021/E022/E023 — validates the pending/resolved outcome syntax of a
//! Branch Property (see README.vision.md "Branch Properties"): `#review:...`
//! marks an outcome as not yet chosen, `#review:passed` resolves it to a
//! named branch.
//!
//! - E021 "Unresolved Branch Property": a task/subtask is marked done while
//!   still carrying a `BranchPending` property. The outcome must be resolved
//!   to one of the property's defined branches before the task can be
//!   considered complete.
//! - E022 "Undefined Branch Outcome": a `BranchResolved` property names a
//!   branch that isn't declared as a `[Properties.X.branch_name]` sub-table
//!   in `mdagile.toml`.
//! - E023 "Plain Branching Property": a property that declares at least one
//!   branch is used in its plain form (`#review`) — neither pending
//!   (`#review:...`) nor resolved (`#review:passed`). Declaring branches at
//!   all means the property *is* a Branching Workflow property, so it must
//!   always carry the pending/resolved syntax, never the bare marker.
//!
//! Branch-specific `subtasks`/`neighbortasks` requirements (declared on those
//! same sub-tables) aren't enforced here -- see
//! [`crate::rules::missing_required_subtasks`] and
//! [`crate::rules::neighbor_tasks`], which merge them in once a property
//! resolves to a known branch.

use crate::config::Config;
use crate::parser::{FileItem, Location, Marker, PropertyForm, Status, TASK_LINE_PREFIX_LEN};
use crate::rules::{ErrorCode, Issue, for_each_node};

pub fn unresolved_branch_property(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        if *node.status() == Status::Done {
            issues.extend(check_unresolved(
                node.markers(),
                node.location(),
                node.indent(),
            ));
        }
    });
    issues
}

/// Checks `markers` for a still-pending (`BranchPending`) branch property and
/// reports it as E021. Status-agnostic: callers decide when this applies —
/// [`unresolved_branch_property`] only calls it for nodes already marked
/// done, while [`crate::rules::check_completable`] calls it unconditionally,
/// since it validates a node that's about to *become* done (still `Todo` at
/// the time of the check — see its own docs).
pub(crate) fn check_unresolved(
    markers: &[Marker],
    location: &Location,
    indent: usize,
) -> Vec<Issue> {
    let pending: Vec<&str> = markers
        .iter()
        .filter_map(|m| match m {
            Marker::Property(p) if p.form == PropertyForm::BranchPending => Some(p.name.as_str()),
            _ => None,
        })
        .collect();

    if pending.is_empty() {
        return vec![];
    }

    let quoted: Vec<String> = pending.iter().map(|p| format!("#{p}:...")).collect();
    vec![Issue {
        location: location.clone(),
        code: ErrorCode::UnresolvedBranchProperty,
        message: format!(
            "Task marked done while branch propert{} {} still unresolved",
            if pending.len() == 1 { "y" } else { "ies" },
            quoted.join(", "),
        ),
        column: indent + 1,
        help: Some(
            "Resolve the outcome to one of its defined branches first, e.g. `#review:passed`."
                .to_string(),
        ),
        data: None,
    }]
}

pub fn undefined_branch_outcome(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        issues.extend(check_undefined_outcome(
            node.markers(),
            node.location(),
            node.indent(),
            config,
        ));
    });
    issues
}

pub(crate) fn check_undefined_outcome(
    markers: &[Marker],
    location: &Location,
    indent: usize,
    config: &Config,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    for marker in markers {
        let Marker::Property(prop) = marker else {
            continue;
        };
        let PropertyForm::BranchResolved(branch) = &prop.form else {
            continue;
        };
        // An undefined property is already reported by E008; don't pile on.
        let Some(prop_config) = config.properties.get(&prop.name) else {
            continue;
        };
        if prop_config.branches.contains_key(branch) {
            continue;
        }
        issues.push(Issue {
            location: location.clone(),
            code: ErrorCode::UndefinedBranchOutcome,
            message: format!(
                "Branch outcome \"{branch}\" is not a defined outcome of property #{}",
                prop.name
            ),
            column: indent + 1,
            help: Some(format!(
                "Declare it with `[Properties.{}.{branch}]` in mdagile.toml, or use one of its \
                 already-defined outcomes.",
                prop.name
            )),
            data: None,
        });
    }
    issues
}

pub fn plain_branching_property(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        issues.extend(check_plain_form(
            node.markers(),
            node.location(),
            node.indent(),
            config,
        ));
    });
    issues
}

/// Checks `markers` for a property used in its plain form (`#review`) even
/// though its config declares at least one branch — i.e. it *is* a Branching
/// Workflow property (see README.vision.md "Branch Properties"), so it must
/// always be written pending (`#review:...`) or resolved (`#review:passed`),
/// never bare. Reported as E023. Reused by [`crate::rules::check_completable`]
/// so `agile task done` also refuses this, not just `agile check`.
pub(crate) fn check_plain_form(
    markers: &[Marker],
    location: &Location,
    indent: usize,
    config: &Config,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    for marker in markers {
        let Marker::Property(prop) = marker else {
            continue;
        };
        if prop.form != PropertyForm::Full {
            continue;
        }
        // An undefined property is already reported by E008; don't pile on.
        let Some(prop_config) = config.properties.get(&prop.name) else {
            continue;
        };
        if prop_config.branches.is_empty() {
            continue;
        }
        // Pick a deterministic example outcome for the help text — branch
        // names come from a `HashMap`, whose iteration order isn't stable.
        let mut branch_names: Vec<&String> = prop_config.branches.keys().collect();
        branch_names.sort();
        let example = branch_names.first().map(|s| s.as_str()).unwrap_or("passed");
        issues.push(Issue {
            location: location.clone(),
            code: ErrorCode::PlainBranchingProperty,
            message: format!(
                "Property #{} declares branches and must be written pending (#{}:...) or \
                 resolved to one of its outcomes, not used plainly",
                prop.name, prop.name
            ),
            column: indent + TASK_LINE_PREFIX_LEN + prop.column,
            help: Some(format!(
                "Write `#{}:...` while still in progress, or resolve it directly, e.g. \
                 `#{}:{example}`.",
                prop.name, prop.name
            )),
            data: None,
        });
    }
    issues
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
