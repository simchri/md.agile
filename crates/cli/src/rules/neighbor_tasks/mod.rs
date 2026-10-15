//! E019 — flags tasks/subtasks whose property requires a "neighbor task"
//! (see README.vision.md "Neighbor Tasks") that isn't present among their
//! direct siblings.
//!
//! A neighbor task is matched the same way a required subtask is: by its
//! quoted raw title (`SubtaskKind::PropertyRequired`). Only [`Subtask`] (not
//! [`Task`]) carries that concept at all, so a `neighbortasks`-carrying
//! property set on a top-level task can never be satisfied — it is always
//! reported missing here, which is exactly the restriction README.vision.md
//! documents ("Properties with Neighbor Tasks can not be set at the top
//! level").

use crate::config::Config;
use crate::parser::{FileItem, Location, Marker, Subtask, SubtaskKind, Task};
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
        // Top-level tasks have no quotable siblings at all (see module docs),
        // so a configured neighbortasks is always reported missing here.
        issues.extend(check_required(
            &task.markers,
            &task.location,
            task.indent,
            &[],
            config,
        ));
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
        .filter(|r| !sibling_titles.contains(&r.as_str()))
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
/// a node, merging across multiple properties and deduplicating.
fn collect_neighbor_titles(markers: &[Marker], config: &Config) -> Vec<String> {
    let mut required: Vec<String> = Vec::new();
    for marker in markers {
        if let Marker::Property(prop) = marker {
            if let Some(prop_config) = config.properties.get(&prop.name) {
                for s in &prop_config.neighbortasks {
                    if !required.contains(s) {
                        required.push(s.clone());
                    }
                }
            }
        }
    }
    required
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
