//! ETA/velocity computation primitives.

use crate::cli::common::find_task_files;
use crate::git;
use crate::parser::{self, FileItem, Status};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const VELOCITY_WINDOW_DAYS: f64 = 90.0;
const VELOCITY_WINDOW_SECS: i64 = (VELOCITY_WINDOW_DAYS as i64) * 24 * 60 * 60;

#[derive(Clone)]
struct NodeSnapshot {
    status: Status,
    weight: f64,
}

/// Estimates current project velocity as weighted completions per day over the
/// last 90 days.
///
/// Returns `None` when there isn't enough git data to produce an estimate.
pub fn estimate_velocity(root: &Path) -> Option<f64> {
    if !git::is_git_repo(root) {
        return None;
    }

    let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    let since_secs = now_secs - VELOCITY_WINDOW_SECS;

    let mut total_completed_weight = 0.0f64;
    let mut saw_any_comparable_pair = false;

    for path in find_task_files(root) {
        let mut commits = git::commits_touching_path(root, &path);
        if commits.len() < 2 {
            continue;
        }

        // git log returns newest -> oldest, but transitions are computed from
        // older snapshot to newer snapshot.
        commits.reverse();
        for pair in commits.windows(2) {
            let old = &pair[0];
            let new = &pair[1];
            if new.timestamp < since_secs {
                continue;
            }

            let Some(old_content) = git::file_content_at_ref(root, &old.sha, &path) else {
                continue;
            };
            let Some(new_content) = git::file_content_at_ref(root, &new.sha, &path) else {
                continue;
            };

            saw_any_comparable_pair = true;

            let old_items = parser::parse(&old_content, path.clone());
            let new_items = parser::parse(&new_content, path.clone());
            total_completed_weight += completion_weight_delta(&old_items, &new_items);
        }
    }

    if !saw_any_comparable_pair {
        return None;
    }

    Some(total_completed_weight / VELOCITY_WINDOW_DAYS)
}

fn completion_weight_delta(old_items: &[FileItem], new_items: &[FileItem]) -> f64 {
    let old_nodes = collect_nodes(old_items);
    let new_nodes = collect_nodes(new_items);

    let mut completed = 0.0f64;
    for (key, new_node) in new_nodes {
        let Some(old_node) = old_nodes.get(&key) else {
            continue;
        };
        if old_node.status == Status::Todo && new_node.status == Status::Done {
            completed += new_node.weight;
        }
    }
    completed
}

fn collect_nodes(items: &[FileItem]) -> HashMap<String, NodeSnapshot> {
    let mut out = HashMap::new();
    let mut top_index = 0usize;

    for item in items {
        let FileItem::Task(task) = item else {
            continue;
        };
        top_index += 1;
        let key = top_index.to_string();
        out.insert(
            key.clone(),
            NodeSnapshot {
                status: task.status.clone(),
                weight: 1.0,
            },
        );
        collect_subtasks(&mut out, &key, &task.children, 2);
    }

    out
}

fn collect_subtasks(
    out: &mut HashMap<String, NodeSnapshot>,
    parent_key: &str,
    children: &[parser::Subtask],
    depth: usize,
) {
    for (idx, child) in children.iter().enumerate() {
        let key = format!("{parent_key}.{}", idx + 1);
        out.insert(
            key.clone(),
            NodeSnapshot {
                status: child.status.clone(),
                weight: 1.0 / (depth as f64),
            },
        );
        collect_subtasks(out, &key, &child.children, depth + 1);
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
