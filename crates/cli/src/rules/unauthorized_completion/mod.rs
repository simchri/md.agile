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
//! transition. Tasks are matched between the two versions by title text, since
//! matching by line number breaks under unrelated edits elsewhere in the file.
//! It is therefore called directly by the CLI/LSP orchestration layer, not as
//! part of `rules::check_all`.

use crate::config::Config;
use crate::parser::{FileItem, Location, Marker, Status, Subtask};
use crate::rules::{ErrorCode, Issue, IssueData};
use std::collections::HashMap;

/// A flattened (title, status, markers, location) view of every task/subtask
/// in a parsed file, used to match nodes across the old/new versions by title.
struct Node<'a> {
    title: &'a str,
    status: &'a Status,
    markers: &'a [Marker],
    location: &'a Location,
}

fn flatten(items: &[FileItem]) -> Vec<Node<'_>> {
    let mut nodes = Vec::new();
    for item in items {
        if let FileItem::Task(task) = item {
            nodes.push(Node {
                title: &task.title,
                status: &task.status,
                markers: &task.markers,
                location: &task.location,
            });
            flatten_subtasks(&task.children, &mut nodes);
        }
    }
    nodes
}

fn flatten_subtasks<'a>(children: &'a [Subtask], nodes: &mut Vec<Node<'a>>) {
    for child in children {
        nodes.push(Node {
            title: &child.title,
            status: &child.status,
            markers: &child.markers,
            location: &child.location,
        });
        flatten_subtasks(&child.children, nodes);
    }
}

/// Returns every `@user`/`@group` name assigned on `markers`.
fn assignment_names(markers: &[Marker]) -> Vec<&str> {
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
fn authorized_users(names: &[&str], config: &Config) -> Vec<String> {
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
/// that transitioned to `[x]` without the current identity (`identity_user`,
/// already resolved to a `[Users.X]` key by the caller) being authorized.
pub fn unauthorized_completion(
    old: Option<&[FileItem]>,
    new: &[FileItem],
    config: &Config,
    identity_user: &str,
) -> Vec<Issue> {
    let old_status_by_title: HashMap<&str, &Status> = old
        .map(|items| {
            flatten(items)
                .into_iter()
                .map(|n| (n.title, n.status))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .collect();

    let mut issues = Vec::new();
    for node in flatten(new) {
        if *node.status != Status::Done {
            continue;
        }
        let was_already_done = old_status_by_title
            .get(node.title)
            .is_some_and(|s| **s == Status::Done);
        if was_already_done {
            continue;
        }

        let names = assignment_names(node.markers);
        if names.is_empty() {
            continue;
        }
        let authorized = authorized_users(&names, config);
        if authorized.iter().any(|a| a == identity_user) {
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
            column: 1,
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
