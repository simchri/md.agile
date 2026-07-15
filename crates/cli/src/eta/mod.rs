//! ETA/velocity computation primitives.

use crate::cli::common::find_task_files;
use crate::git;
use crate::parser::{self, FileItem, Status};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const VELOCITY_WINDOW_DAYS: f64 = 90.0;
const SECONDS_PER_DAY: f64 = 24.0 * 60.0 * 60.0;
const VELOCITY_WINDOW_SECS: i64 = (VELOCITY_WINDOW_DAYS as i64) * 24 * 60 * 60;

#[derive(Default, Clone, Copy)]
struct DepthStatusCounts {
    todo: u32,
    done: u32,
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
    let mut min_timestamp: Option<i64> = None;
    let mut max_timestamp: Option<i64> = None;

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
            min_timestamp = Some(min_timestamp.map_or(old.timestamp, |t| t.min(old.timestamp)));
            max_timestamp = Some(max_timestamp.map_or(new.timestamp, |t| t.max(new.timestamp)));

            let old_items = parser::parse(&old_content, path.clone());
            let new_items = parser::parse(&new_content, path.clone());
            total_completed_weight += completion_weight_delta(&old_items, &new_items);
        }
    }

    if !saw_any_comparable_pair {
        return None;
    }

    let span_secs = (max_timestamp? - min_timestamp?).max(0) as f64;
    let span_days = span_secs / SECONDS_PER_DAY;
    if span_days <= 0.0 {
        return None;
    }

    Some(total_completed_weight / span_days)
}

fn completion_weight_delta(old_items: &[FileItem], new_items: &[FileItem]) -> f64 {
    let old_counts = collect_done_counts_by_depth(old_items);
    let new_counts = collect_done_counts_by_depth(new_items);

    let mut completed_weight = 0.0f64;
    for (depth, new) in new_counts {
        let old = old_counts.get(&depth).copied().unwrap_or_default();
        let additional_done = new.done.saturating_sub(old.done);
        let completed_nodes = additional_done.min(old.todo) as f64;
        if completed_nodes <= 0.0 {
            continue;
        }
        completed_weight += completed_nodes * weight_for_depth(depth);
    }
    completed_weight
}

fn collect_done_counts_by_depth(items: &[FileItem]) -> HashMap<usize, DepthStatusCounts> {
    let mut out = HashMap::new();
    for item in items {
        let FileItem::Task(task) = item else {
            continue;
        };
        let level = out.entry(1).or_insert_with(DepthStatusCounts::default);
        if task.status == Status::Todo {
            level.todo += 1;
        } else if task.status == Status::Done {
            level.done += 1;
        }
        collect_subtasks(&mut out, &task.children, 2);
    }
    out
}

fn collect_subtasks(
    out: &mut HashMap<usize, DepthStatusCounts>,
    children: &[parser::Subtask],
    depth: usize,
) {
    for child in children {
        let level = out.entry(depth).or_insert_with(DepthStatusCounts::default);
        if child.status == Status::Todo {
            level.todo += 1;
        } else if child.status == Status::Done {
            level.done += 1;
        }
        collect_subtasks(out, &child.children, depth + 1);
    }
}

fn weight_for_depth(depth: usize) -> f64 {
    1.0 / (depth as f64)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
