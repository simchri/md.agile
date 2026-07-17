//! ETA/velocity computation primitives.

use crate::cli::common::find_task_files;
use crate::git;
use crate::history_cache;
use crate::parser::{self, FileItem, Status};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use textplots::{Chart, Plot, Shape};

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

#[derive(Debug, Clone, PartialEq)]
pub struct TodoDonePlotPoint {
    pub date: String,
    pub total_weight: f64,
    pub done_weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TodoDonePlot {
    pub milestone_name: String,
    pub points: Vec<TodoDonePlotPoint>,
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

    let mut total_completed_weight = 0.0;
    let mut comparable_pairs = 0usize;
    let mut completion_events = 0usize;
    let mut min_timestamp: Option<i64> = None;
    let mut max_timestamp: Option<i64> = None;

    let cache = history_cache::update(root)?;
    for (old, new) in cache.entries.iter().zip(cache.entries.iter().skip(1)) {
        if new.commit_timestamp < since_secs {
            continue;
        }
        comparable_pairs += 1;
        let old_for_span = old.commit_timestamp.max(since_secs);
        min_timestamp = Some(min_timestamp.map_or(old_for_span, |t| t.min(old_for_span)));
        max_timestamp =
            Some(max_timestamp.map_or(new.commit_timestamp, |t| t.max(new.commit_timestamp)));
        total_completed_weight += new.completed_weight_from_previous;
        completion_events += new.completion_events_from_previous;
    }

    if let Some(latest_entry) = cache.entries.last() {
        let (delta_weight, delta_events, worktree_changed) =
            worktree_completion_delta(root, &latest_entry.commit_hash);
        if worktree_changed {
            comparable_pairs += 1;
            let old_for_span = latest_entry.commit_timestamp.max(since_secs);
            min_timestamp = Some(min_timestamp.map_or(old_for_span, |t| t.min(old_for_span)));
            max_timestamp = Some(max_timestamp.map_or(now_secs, |t| t.max(now_secs)));
            total_completed_weight += delta_weight;
            completion_events += delta_events;
        }
    }

    if comparable_pairs == 0 {
        return None;
    }

    fn worktree_completion_delta(root: &Path, latest_commit_sha: &str) -> (f64, usize, bool) {
        let mut total_completed_weight = 0.0;
        let mut completion_events = 0usize;
        let mut worktree_changed = false;

        for path in find_task_files(root) {
            let Some(latest_content) = git::file_content_at_ref(root, latest_commit_sha, &path)
            else {
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
            worktree_changed = true;

            let old_items = parser::parse(&latest_content, path.clone());
            let new_items = parser::parse(&worktree_content, path.clone());
            let (delta_weight, delta_events) = completion_weight_delta(&old_items, &new_items);
            total_completed_weight += delta_weight;
            completion_events += delta_events;
        }

        (total_completed_weight, completion_events, worktree_changed)
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

pub fn build_todo_done_plot(root: &Path, milestone_rank: usize) -> Result<TodoDonePlot, String> {
    if !git::is_git_repo(root) {
        return Err("`agile when --plot` requires a git repository".to_string());
    }
    if milestone_rank == 0 {
        return Err("milestone rank must be >= 1".to_string());
    }

    let milestone_name = milestone_name_for_rank(root, milestone_rank)
        .ok_or_else(|| format!("milestone rank {milestone_rank} does not exist"))?;
    let cache = history_cache::update(root)
        .ok_or_else(|| "could not read or build history cache for plotting".to_string())?;

    let mut points = Vec::new();
    for entry in &cache.entries {
        let (total_weight, done_weight, found_milestone) =
            weights_until_milestone_at_ref(root, &entry.commit_hash, &milestone_name);
        if !found_milestone {
            continue;
        }
        points.push(TodoDonePlotPoint {
            date: entry.commit_date.clone(),
            total_weight,
            done_weight,
        });
    }

    if points.is_empty() {
        return Err(format!(
            "milestone '{}' is not present in comparable history entries",
            milestone_name
        ));
    }

    Ok(TodoDonePlot {
        milestone_name,
        points,
    })
}

pub fn render_todo_done_plot(plot: &TodoDonePlot, ascii: bool) -> String {
    let sampled = downsample_plot_points(&plot.points, 96);

    let mut out = String::new();
    out.push_str(&format!("milestone: {}\n", plot.milestone_name));
    if ascii {
        out.push_str(&render_ascii_plot(&sampled));
    } else {
        out.push_str("legend: textplots line1=total_weight line2=done_weight\n");
        out.push_str(&render_textplots_chart(&sampled));
    }

    let start_date = sampled
        .first()
        .map(|p| p.date.clone())
        .unwrap_or_else(|| "n/a".to_string());
    let end_date = sampled
        .last()
        .map(|p| p.date.clone())
        .unwrap_or_else(|| "n/a".to_string());
    out.push_str(&format!("         {} .. {}\n", start_date, end_date));

    if let Some(last) = sampled.last() {
        out.push_str(&format!(
            "latest: total_weight={:.2}, done_weight={:.2}\n",
            last.total_weight, last.done_weight
        ));
    }
    out
}

fn render_textplots_chart(points: &[TodoDonePlotPoint]) -> String {
    let total_series: Vec<(f32, f32)> = points
        .iter()
        .enumerate()
        .map(|(i, p)| (i as f32, p.total_weight as f32))
        .collect();
    let done_series: Vec<(f32, f32)> = points
        .iter()
        .enumerate()
        .map(|(i, p)| (i as f32, p.done_weight as f32))
        .collect();
    let xmax = (points.len().saturating_sub(1).max(1)) as f32;
    let ymax = points
        .iter()
        .map(|p| p.total_weight.max(p.done_weight))
        .fold(0.0, f64::max)
        .max(1.0) as f32;

    let total_shape = Shape::Lines(total_series.as_slice());
    let done_shape = Shape::Lines(done_series.as_slice());
    let mut chart = Chart::new_with_y_range(120, 36, 0.0, xmax, 0.0, ymax);
    let chart_ref = chart.lineplot(&total_shape).lineplot(&done_shape);
    chart_ref.axis();
    chart_ref.figures();
    format!("{chart_ref}\n")
}

fn render_ascii_plot(points: &[TodoDonePlotPoint]) -> String {
    let width = points.len().max(1);
    let height = 12usize;
    let y_max = points
        .iter()
        .map(|p| p.total_weight.max(p.done_weight))
        .fold(0.0, f64::max)
        .max(1.0);
    let mut grid = vec![vec![' '; width]; height];
    for (x, p) in points.iter().enumerate() {
        let total_row = y_to_row(p.total_weight, y_max, height);
        let done_row = y_to_row(p.done_weight, y_max, height);
        place_marker(&mut grid, total_row, x, '*', 'X');
        place_marker(&mut grid, done_row, x, 'o', 'X');
    }

    let mut out = String::new();
    out.push_str("legend: * total_weight, o done_weight, X overlap\n");
    for (row_idx, row) in grid.iter().enumerate() {
        let y = if height == 1 {
            0.0
        } else {
            y_max * ((height - 1 - row_idx) as f64) / ((height - 1) as f64)
        };
        out.push_str(&format!("{:>7.2} |", y));
        for c in row {
            out.push(*c);
        }
        out.push('\n');
    }
    out.push_str("         +");
    out.push_str(&"-".repeat(width));
    out.push('\n');
    out
}

fn place_marker(
    grid: &mut [Vec<char>],
    row: usize,
    col: usize,
    marker: char,
    overlap_marker: char,
) {
    let current = grid[row][col];
    grid[row][col] = if current == ' ' || current == marker {
        marker
    } else {
        overlap_marker
    };
}

fn y_to_row(value: f64, y_max: f64, height: usize) -> usize {
    if height <= 1 {
        return 0;
    }
    let clamped = if y_max <= 0.0 {
        0.0
    } else {
        (value / y_max).clamp(0.0, 1.0)
    };
    ((1.0 - clamped) * (height as f64 - 1.0)).round() as usize
}

fn downsample_plot_points(
    points: &[TodoDonePlotPoint],
    max_points: usize,
) -> Vec<TodoDonePlotPoint> {
    if points.len() <= max_points || max_points == 0 {
        return points.to_vec();
    }
    if max_points == 1 {
        return vec![points[points.len() - 1].clone()];
    }
    let mut out = Vec::with_capacity(max_points);
    for i in 0..max_points {
        let idx = i * (points.len() - 1) / (max_points - 1);
        out.push(points[idx].clone());
    }
    out
}

fn milestone_name_for_rank(root: &Path, milestone_rank: usize) -> Option<String> {
    let mut milestones = Vec::new();
    for path in find_task_files(root) {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let items = parser::parse(&content, path);
        for item in items {
            let FileItem::Milestone(m) = item else {
                continue;
            };
            milestones.push(m.name);
        }
    }
    milestones.get(milestone_rank - 1).cloned()
}

fn weights_until_milestone_at_ref(
    root: &Path,
    git_ref: &str,
    target_milestone: &str,
) -> (f64, f64, bool) {
    let mut paths = git::task_files_at_ref(root, git_ref);
    paths.sort();

    let mut total_weight = 0.0;
    let mut done_weight = 0.0;
    for path in paths {
        let Some(content) = git::file_content_at_ref(root, git_ref, &path) else {
            continue;
        };
        let items = parser::parse(&content, path);
        for item in items {
            match item {
                FileItem::Task(task) => {
                    let weight = task_total_weight(&task);
                    total_weight += weight;
                    match task.status {
                        Status::Done | Status::Cancelled => done_weight += weight,
                        Status::Todo => {}
                    }
                }
                FileItem::Milestone(m) => {
                    if m.name == target_milestone {
                        return (total_weight, done_weight, true);
                    }
                }
            }
        }
    }

    (0.0, 0.0, false)
}

fn task_total_weight(task: &parser::Task) -> f64 {
    1.0 + subtasks_total_weight(&task.children, 2)
}

fn subtasks_total_weight(children: &[parser::Subtask], depth: usize) -> f64 {
    children
        .iter()
        .map(|c| (1.0 / depth as f64) + subtasks_total_weight(&c.children, depth + 1))
        .sum()
}

pub(crate) fn completion_weight_delta(
    old_items: &[FileItem],
    new_items: &[FileItem],
) -> (f64, usize) {
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
