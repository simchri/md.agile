//! ETA/velocity computation primitives.

use crate::cli::common::find_task_files;
use crate::git;
use crate::parser::{self, FileItem, Status};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DEFAULT_VELOCITY_WINDOW_DAYS: u32 = 90;
const SECONDS_PER_DAY: f64 = 24.0 * 60.0 * 60.0;

#[derive(Debug, Clone, PartialEq)]
struct FlatNode {
    path: Vec<String>,
    status: Status,
    depth: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VelocityEstimate {
    pub weight_per_day: f64,
    pub completed_weight: f64,
    pub span_days: f64,
    pub comparable_pairs: usize,
    pub completion_events: usize,
}

/// Estimates current project velocity as weighted completions per day over the
/// last 90 days.
///
/// Returns `None` when there isn't enough git data to produce an estimate.
pub fn estimate_velocity(root: &Path) -> Option<f64> {
    estimate_velocity_with_window(root, DEFAULT_VELOCITY_WINDOW_DAYS)
}

/// Estimates velocity over a caller-provided trailing window (in days).
pub fn estimate_velocity_with_window(root: &Path, window_days: u32) -> Option<f64> {
    estimate_velocity_details_with_window(root, window_days).map(|v| v.weight_per_day)
}

/// Like [`estimate_velocity`], but returns additional metadata useful for
/// diagnostics and future confidence/error-margin reporting.
pub fn estimate_velocity_details(root: &Path) -> Option<VelocityEstimate> {
    estimate_velocity_details_with_window(root, DEFAULT_VELOCITY_WINDOW_DAYS)
}

/// Like [`estimate_velocity_with_window`], but returns additional metadata.
pub fn estimate_velocity_details_with_window(
    root: &Path,
    window_days: u32,
) -> Option<VelocityEstimate> {
    if !git::is_git_repo(root) {
        return None;
    }
    if window_days == 0 {
        return None;
    }

    let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    let window_secs = Duration::from_secs(u64::from(window_days) * 24 * 60 * 60).as_secs() as i64;
    let since_secs = now_secs - window_secs;

    let mut total_completed_weight = 0.0f64;
    let mut comparable_pairs = 0usize;
    let mut completion_events = 0usize;
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

            comparable_pairs += 1;
            let old_for_span = old.timestamp.max(since_secs);
            min_timestamp = Some(min_timestamp.map_or(old_for_span, |t| t.min(old_for_span)));
            max_timestamp = Some(max_timestamp.map_or(new.timestamp, |t| t.max(new.timestamp)));

            let old_items = parser::parse(&old_content, path.clone());
            let new_items = parser::parse(&new_content, path.clone());
            let (delta_weight, delta_events) = completion_weight_delta(&old_items, &new_items);
            total_completed_weight += delta_weight;
            completion_events += delta_events;
        }
    }

    if comparable_pairs == 0 {
        return None;
    }

    let span_secs = (max_timestamp? - min_timestamp?).max(0) as f64;
    let span_days = span_secs / SECONDS_PER_DAY;
    if span_days <= 0.0 {
        return None;
    }

    Some(VelocityEstimate {
        weight_per_day: total_completed_weight / span_days,
        completed_weight: total_completed_weight,
        span_days,
        comparable_pairs,
        completion_events,
    })
}

fn completion_weight_delta(old_items: &[FileItem], new_items: &[FileItem]) -> (f64, usize) {
    let old_nodes = flatten_nodes(old_items);
    let new_nodes = flatten_nodes(new_items);

    // Same path may legitimately occur multiple times (duplicate sibling titles).
    // Match by path+occurrence index (document order), same strategy as E013.
    let mut old_status_by_path: HashMap<Vec<String>, Vec<Status>> = HashMap::new();
    for node in old_nodes {
        old_status_by_path
            .entry(node.path)
            .or_default()
            .push(node.status);
    }

    let mut occurrence_index: HashMap<Vec<String>, usize> = HashMap::new();
    let mut completed_weight = 0.0f64;
    let mut completion_events = 0usize;
    for new in new_nodes {
        let idx = occurrence_index.entry(new.path.clone()).or_insert(0);
        let old_status = old_status_by_path
            .get(&new.path)
            .and_then(|v| v.get(*idx))
            .cloned();
        *idx += 1;

        if old_status == Some(Status::Todo) && new.status == Status::Done {
            completion_events += 1;
            completed_weight += weight_for_depth(new.depth);
        }
    }
    (completed_weight, completion_events)
}

fn flatten_nodes(items: &[FileItem]) -> Vec<FlatNode> {
    let mut out = Vec::new();
    for item in items {
        let FileItem::Task(task) = item else {
            continue;
        };
        let path = vec![task.title.clone()];
        out.push(FlatNode {
            path: path.clone(),
            status: task.status.clone(),
            depth: 1,
        });
        flatten_subtasks(&mut out, &path, &task.children, 2);
    }
    out
}

fn flatten_subtasks(
    out: &mut Vec<FlatNode>,
    parent_path: &[String],
    children: &[parser::Subtask],
    depth: usize,
) {
    for child in children {
        let mut path = parent_path.to_vec();
        path.push(child.title.clone());
        out.push(FlatNode {
            path: path.clone(),
            status: child.status.clone(),
            depth,
        });
        flatten_subtasks(out, &path, &child.children, depth + 1);
    }
}

fn weight_for_depth(depth: usize) -> f64 {
    1.0 / (depth as f64)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
