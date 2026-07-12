//! E013 — flags tasks/subtasks marked done by someone not authorized to do so.
//!
//! "Authorized" means: the current git identity (already resolved to a
//! `[Users.X]` config key by the caller) is directly assigned to the task via
//! `@user`, or is a member of a `@group` assigned to the task. Tasks with no
//! assignment markers at all are never flagged — this check only nudges
//! towards respecting *existing* assignments, it doesn't mandate them.
//!
//! This rule differs from the others in `rules::` in that it needs *two*
//! versions of the file (the last-committed `HEAD` version and the current
//! working-copy version) to detect a `[ ] -> [x]` (or no-prior-version -> `[x]`)
//! transition. Tasks are matched between the two versions by their full
//! ancestor-title path (not line number, which breaks under unrelated edits
//! elsewhere in the file, and not bare title alone, which collides whenever
//! two different tasks have same-titled subtasks — e.g. `#property`-required
//! subtasks, which by design reuse the same literal title across every task
//! carrying that property). It is therefore called directly by the CLI/LSP
//! orchestration layer, not as part of `rules::check_all`.

use crate::config::Config;
use crate::parser::{FileItem, Location, Marker, Status, Subtask};
use crate::rules::{ErrorCode, Issue, IssueData, ResolvedIdentity};
use std::collections::HashMap;

/// A flattened (path, status, markers, location) view of every task/subtask
/// in a parsed file, used to match nodes across the old/new versions.
///
/// `path` is the full ancestor chain of titles from the root task down to
/// (and including) this node's own title — not just the bare title. Two
/// different tasks commonly have same-titled subtasks (e.g. `#property`
/// required subtasks reuse the same literal title on every task carrying
/// that property), so matching by bare title alone would conflate them;
/// scoping by the full path disambiguates same-titled subtasks that live
/// under different parents.
struct Node<'a> {
    path: Vec<&'a str>,
    status: &'a Status,
    markers: &'a [Marker],
    location: &'a Location,
    indent: usize,
}

fn flatten(items: &[FileItem]) -> Vec<Node<'_>> {
    let mut nodes = Vec::new();
    for item in items {
        if let FileItem::Task(task) = item {
            let path = vec![task.title.as_str()];
            nodes.push(Node {
                path: path.clone(),
                status: &task.status,
                markers: &task.markers,
                location: &task.location,
                indent: task.indent,
            });
            flatten_subtasks(&task.children, &path, &mut nodes);
        }
    }
    nodes
}

fn flatten_subtasks<'a>(
    children: &'a [Subtask],
    parent_path: &[&'a str],
    nodes: &mut Vec<Node<'a>>,
) {
    for child in children {
        let mut path = parent_path.to_vec();
        path.push(child.title.as_str());
        nodes.push(Node {
            path: path.clone(),
            status: &child.status,
            markers: &child.markers,
            location: &child.location,
            indent: child.indent,
        });
        flatten_subtasks(&child.children, &path, nodes);
    }
}

/// Returns every `@user`/`@group` name assigned on `markers`.
pub(crate) fn assignment_names(markers: &[Marker]) -> Vec<&str> {
    markers
        .iter()
        .filter_map(|m| match m {
            Marker::Assignment(a) => Some(a.name.as_str()),
            _ => None,
        })
        .collect()
}

/// Expands assignment names into the set of concrete `[Users.X]` keys they
/// authorize: direct user names, plus every member of any assigned group.
pub(crate) fn authorized_users(names: &[&str], config: &Config) -> Vec<String> {
    let mut authorized: Vec<String> = Vec::new();
    for &name in names {
        if config.users.contains_key(name) && !authorized.iter().any(|a| a == name) {
            authorized.push(name.to_string());
        }
        if let Some(group) = config.groups.get(name) {
            for member in &group.members {
                if !authorized.iter().any(|a| a == member) {
                    authorized.push(member.clone());
                }
            }
        }
    }
    authorized.sort();
    authorized
}

/// Compares the `old` (HEAD, `None` if no committed version exists) and `new`
/// (working copy) versions of a file's parsed items, flagging every task/subtask
/// that transitioned to `[x]` without `identity` being authorized.
///
/// `identity: ResolvedIdentity::Unrecognized` (an identity was determined but
/// doesn't match any configured user) is always unauthorized for any assigned
/// task — the caller must not call this function at all if it wants a full
/// skip (e.g. when no identity could be determined whatsoever).
pub fn unauthorized_completion(
    old: Option<&[FileItem]>,
    new: &[FileItem],
    config: &Config,
    identity: &ResolvedIdentity,
) -> Vec<Issue> {
    // Two different nodes can share the exact same ancestor-title path --
    // e.g. duplicate sibling titles, which nothing in the parser/checker
    // forbids. Group old statuses by path (preserving document order within
    // each group) so that same-path occurrences are matched positionally
    // (1st old occurrence <-> 1st new occurrence, 2nd <-> 2nd, ...) instead
    // of being collapsed into a single entry that could misattribute a
    // "was already done" status across unrelated nodes.
    let mut old_status_by_path: HashMap<Vec<&str>, Vec<&Status>> = HashMap::new();
    if let Some(items) = old {
        for n in flatten(items) {
            old_status_by_path.entry(n.path).or_default().push(n.status);
        }
    }

    let mut occurrence_index: HashMap<Vec<&str>, usize> = HashMap::new();
    let mut issues = Vec::new();
    for node in flatten(new) {
        let idx = occurrence_index.entry(node.path.clone()).or_insert(0);
        let old_status = old_status_by_path.get(&node.path).and_then(|v| v.get(*idx));
        *idx += 1;

        if *node.status != Status::Done {
            continue;
        }
        let was_already_done = old_status.is_some_and(|s| **s == Status::Done);
        if was_already_done {
            continue;
        }

        let names = assignment_names(node.markers);
        if names.is_empty() {
            continue;
        }
        let authorized = authorized_users(&names, config);
        let is_authorized = match identity {
            ResolvedIdentity::Known(user) => authorized.iter().any(|a| a == user),
            ResolvedIdentity::Unrecognized => false,
        };
        if is_authorized {
            continue;
        }

        issues.push(Issue {
            location: node.location.clone(),
            code: ErrorCode::UnauthorizedCompletion,
            message: format!(
                "Task marked done by an unauthorized user; only {} may complete it",
                authorized
                    .iter()
                    .map(|a| format!("\"{a}\""))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            column: node.indent + 1,
            help: Some(
                "Only assigned users (or members of an assigned group) should mark this task done."
                    .to_string(),
            ),
            data: Some(IssueData::UnauthorizedCompletion { authorized }),
        });
    }
    issues
}

#[cfg(test)]
mod tests;
