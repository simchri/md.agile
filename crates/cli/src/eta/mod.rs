//! ETA/velocity computation primitives.

use crate::cli::common::find_task_files;
use crate::git;
use crate::parser::{self, FileItem, Status};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DEFAULT_VELOCITY_WINDOW_DAYS: u32 = 90;
const SECONDS_PER_DAY: f64 = 24.0 * 60.0 * 60.0;

#[derive(Debug, Clone, PartialEq)]
struct FlatNode {
    key: TransitionKey,
    status: Status,
    depth: usize,
    indent: usize,
    title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransitionKey {
    pub path: Vec<String>,
    pub occurrence: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusTransition {
    pub key: TransitionKey,
    pub old_status: Option<Status>,
    pub new_status: Status,
    pub depth: usize,
    pub indent: usize,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FallbackSignature {
    depth: usize,
    title: String,
    parent_title: Option<String>,
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
        if commits.is_empty() {
            continue;
        }
        let latest_commit = commits.first().cloned();

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

        // Include the current working tree as the latest state so uncommitted
        // changes contribute to velocity.
        if let Some(latest) = latest_commit {
            let Some(latest_content) = git::file_content_at_ref(root, &latest.sha, &path) else {
                continue;
            };
            let worktree_path = root.join(&path);
            let worktree_content = match std::fs::read_to_string(&worktree_path) {
                Ok(content) => content,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(_) => continue,
            };
            if worktree_content == latest_content {
                continue;
            }

            comparable_pairs += 1;
            let old_for_span = latest.timestamp.max(since_secs);
            min_timestamp = Some(min_timestamp.map_or(old_for_span, |t| t.min(old_for_span)));
            max_timestamp = Some(max_timestamp.map_or(now_secs, |t| t.max(now_secs)));

            let old_items = parser::parse(&latest_content, path.clone());
            let new_items = parser::parse(&worktree_content, path.clone());
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
    let transitions = status_transitions(old_items, new_items);
    let mut completed_weight = 0.0f64;
    let mut completion_events = 0usize;
    for t in transitions {
        if t.old_status == Some(Status::Todo) && t.new_status == Status::Done {
            completion_events += 1;
            completed_weight += weight_for_depth(t.depth);
        }
    }
    (completed_weight, completion_events)
}

/// Returns path+occurrence-matched status transitions from `old_items` to
/// `new_items`.
pub fn status_transitions(old_items: &[FileItem], new_items: &[FileItem]) -> Vec<StatusTransition> {
    let old_nodes = flatten_nodes(old_items);
    let new_nodes = flatten_nodes(new_items);
    let old_by_key: HashMap<TransitionKey, FlatNode> =
        old_nodes.into_iter().map(|n| (n.key.clone(), n)).collect();

    let mut matched_old = HashSet::new();
    let mut transitions = Vec::with_capacity(new_nodes.len());
    let mut unmatched_new = Vec::new();
    for new in new_nodes {
        let old_status = old_by_key.get(&new.key).map(|old| {
            matched_old.insert(new.key.clone());
            old.status.clone()
        });
        if old_status.is_none() {
            unmatched_new.push(new.clone());
        }
        transitions.push(StatusTransition {
            key: new.key,
            old_status,
            new_status: new.status,
            depth: new.depth,
            indent: new.indent,
            title: new.title,
        });
    }

    // Fallback matcher: when strict path+occurrence fails (e.g. ancestor title
    // churn), match uniquely by local structural signature.
    let mut old_unmatched_by_sig: HashMap<FallbackSignature, Vec<TransitionKey>> = HashMap::new();
    for (key, old) in &old_by_key {
        if matched_old.contains(key) {
            continue;
        }
        old_unmatched_by_sig
            .entry(fallback_signature(old))
            .or_default()
            .push(key.clone());
    }

    let mut consumed_old_fallback = HashSet::new();
    for t in &mut transitions {
        if t.old_status.is_some() {
            continue;
        }
        let sig = FallbackSignature {
            depth: t.depth,
            title: t.title.clone(),
            parent_title: parent_title_from_path(&t.key.path),
        };
        let Some(candidates) = old_unmatched_by_sig.get(&sig) else {
            continue;
        };
        // Conservative: only use fallback when there is one unambiguous old node.
        let available: Vec<&TransitionKey> = candidates
            .iter()
            .filter(|k| !consumed_old_fallback.contains(*k))
            .collect();
        if available.len() != 1 {
            continue;
        }
        let key = available[0];
        let Some(old) = old_by_key.get(key) else {
            continue;
        };
        consumed_old_fallback.insert(key.clone());
        t.old_status = Some(old.status.clone());
    }

    transitions
}

fn flatten_nodes(items: &[FileItem]) -> Vec<FlatNode> {
    let mut raw = Vec::new();
    for item in items {
        let FileItem::Task(task) = item else {
            continue;
        };
        let path = vec![task.title.clone()];
        raw.push((
            path.clone(),
            task.status.clone(),
            1usize,
            task.indent,
            task.title.clone(),
        ));
        flatten_subtasks(&mut raw, &path, &task.children, 2);
    }

    let mut occurrence_index: HashMap<Vec<String>, usize> = HashMap::new();
    raw.into_iter()
        .map(|(path, status, depth, indent, title)| {
            let occurrence = occurrence_index.entry(path.clone()).or_insert(0);
            let key = TransitionKey {
                path: path.clone(),
                occurrence: *occurrence,
            };
            *occurrence += 1;
            FlatNode {
                key,
                status,
                depth,
                indent,
                title,
            }
        })
        .collect()
}

fn flatten_subtasks(
    out: &mut Vec<(Vec<String>, Status, usize, usize, String)>,
    parent_path: &[String],
    children: &[parser::Subtask],
    depth: usize,
) {
    for child in children {
        let mut path = parent_path.to_vec();
        path.push(child.title.clone());
        out.push((
            path.clone(),
            child.status.clone(),
            depth,
            child.indent,
            child.title.clone(),
        ));
        flatten_subtasks(out, &path, &child.children, depth + 1);
    }
}

fn weight_for_depth(depth: usize) -> f64 {
    1.0 / (depth as f64)
}

fn fallback_signature(node: &FlatNode) -> FallbackSignature {
    FallbackSignature {
        depth: node.depth,
        title: node.title.clone(),
        parent_title: parent_title_from_path(&node.key.path),
    }
}

fn parent_title_from_path(path: &[String]) -> Option<String> {
    path.len().checked_sub(2).map(|idx| path[idx].clone())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
