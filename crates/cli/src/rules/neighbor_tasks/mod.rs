//! E019 — flags tasks/subtasks whose property requires a "neighbor task"
//! (see README.vision.md "Neighbor Tasks") that isn't present among their
//! direct siblings.
//!
//! A neighbor task is matched the same way a required subtask is: by its
//! quoted raw title (`SubtaskKind::PropertyRequired`). Only [`Subtask`] (not
//! [`Task`]) carries that concept at all, so a `neighbortasks`-carrying
//! property set on a top-level task can never be satisfied there — but that
//! case is reported as its own, more specific error instead: see
//! [`crate::rules::neighbor_task_on_top_level`] (E020), which `check_all`
//! runs *before* this rule. This rule therefore skips top-level tasks
//! entirely, so the two checks never both fire for the same task.

use crate::config::Config;
use crate::parser::{FileItem, Location, Marker, Subtask, SubtaskKind, Task};
use crate::rules::branch_properties::merge_property_and_branch_strings;
use crate::rules::required_title::matches_required_title;
use crate::rules::{ErrorCode, Issue, IssueData};

pub fn neighbor_tasks(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();

    let top_level_tasks: Vec<&Task> = items
        .iter()
        .filter_map(|item| match item {
            FileItem::Task(t) => Some(t),
            FileItem::Milestone(_) => None,
        })
        .collect();

    for task in &top_level_tasks {
        // A top-level task's own neighbortasks (if any) is E020's concern,
        // not this rule's (see module docs) — only recurse into children.
        check_sibling_list(&task.children, config, &mut issues);
    }

    issues
}

/// Checks every subtask in one sibling list (one parent's direct children)
/// against the other members of that same list, then recurses into each
/// subtask's own children (its own sibling list, one level down).
fn check_sibling_list(siblings: &[Subtask], config: &Config, issues: &mut Vec<Issue>) {
    for (i, sub) in siblings.iter().enumerate() {
        let other_titles: Vec<&str> = siblings
            .iter()
            .enumerate()
            .filter(|(j, s)| *j != i && s.kind == SubtaskKind::PropertyRequired)
            .filter_map(|(_, s)| s.raw_title.as_deref())
            .collect();
        issues.extend(check_required(
            &sub.markers,
            &sub.location,
            sub.indent,
            &other_titles,
            config,
        ));
        check_sibling_list(&sub.children, config, issues);
    }
}

fn check_required(
    markers: &[Marker],
    location: &Location,
    indent: usize,
    sibling_titles: &[&str],
    config: &Config,
) -> Vec<Issue> {
    let required = collect_neighbor_titles(markers, config);
    if required.is_empty() {
        return vec![];
    }

    let missing: Vec<String> = required
        .into_iter()
        .filter(|r| {
            !sibling_titles
                .iter()
                .any(|title| matches_required_title(r, title))
        })
        .collect();

    if missing.is_empty() {
        return vec![];
    }

    let missing_quoted: Vec<String> = missing.iter().map(|s| format!("\"{s}\"")).collect();
    vec![Issue {
        location: location.clone(),
        code: ErrorCode::MissingNeighborTask,
        message: format!(
            "Missing required neighbor task{}: {}",
            if missing.len() == 1 { "" } else { "s" },
            missing_quoted.join(", "),
        ),
        column: indent + 1,
        help: Some(
            "Add the missing quoted sibling task at the same level, e.g. \
             `- [ ] \"do the back end implementation also!\"`"
                .to_string(),
        ),
        data: Some(IssueData::MissingNeighborTask { missing }),
    }]
}

/// Collects all neighbor-task titles contributed by the property markers on
/// a node, merging across multiple properties and deduplicating. Exposed at
/// `pub(crate)` so [`crate::rules::neighbor_task_on_top_level`] (E020) can
/// detect, by the same logic, whether a top-level task carries a property
/// that requires neighbor tasks at all.
pub(crate) fn collect_neighbor_titles(markers: &[Marker], config: &Config) -> Vec<String> {
    merge_property_and_branch_strings(markers, config, |p| &p.neighbortasks, |b| &b.neighbortasks)
}

/// Collects the *names* of properties on `markers` whose config declares
/// `neighbortasks` -- on the base property itself, or on *any* of its
/// declared branches (see README.vision.md "Branch Properties"): a
/// branch-specific `neighbortasks` requirement is just as structurally
/// unsatisfiable on a top-level task as the base property's own, regardless
/// of which outcome (if any) is currently chosen. Deduplicated. Used by
/// [`crate::rules::neighbor_task_on_top_level`] (E020) to name the offending
/// propert(y/ies) in its message.
pub(crate) fn property_names_requiring_neighbors(
    markers: &[Marker],
    config: &Config,
) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for marker in markers {
        if let Marker::Property(prop) = marker {
            if let Some(prop_config) = config.properties.get(&prop.name) {
                let requires_neighbors = !prop_config.neighbortasks.is_empty()
                    || prop_config
                        .branches
                        .values()
                        .any(|b| !b.neighbortasks.is_empty());
                if requires_neighbors && !names.contains(&prop.name) {
                    names.push(prop.name.clone());
                }
            }
        }
    }
    names
}

/// Collects all neighbor-task titles required by any member of `siblings`'
/// own property markers — i.e. the titles any of them declares as a required
/// neighbor via `neighbortasks`. Shared with
/// [`crate::rules::unrequired_quoted_subtask`] so a quoted sibling satisfying
/// someone else's `neighbortasks` isn't also flagged there as an "unrequired"
/// quoted subtask (E011): that rule only knows about `subtasks` required by
/// the shared *parent's* own properties, not about `neighbortasks` required
/// by a *sibling's* property, since the two are declared on different nodes.
pub(crate) fn required_by_siblings(siblings: &[Subtask], config: &Config) -> Vec<String> {
    let mut required: Vec<String> = Vec::new();
    for sub in siblings {
        for title in collect_neighbor_titles(&sub.markers, config) {
            if !required.contains(&title) {
                required.push(title);
            }
        }
    }
    required
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
