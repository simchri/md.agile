//! ETA/velocity computation primitives.

use crate::cli::common::find_task_files;
use crate::git;
use crate::history_cache;
use crate::parser::{self, FileItem, Status};
use rgb::RGB8;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use textplots::{Chart, ColorPlot, LabelBuilder, LabelFormat, Shape};

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
    pub total_count: usize,
    pub done_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TodoDonePlot {
    pub milestone_name: String,
    pub points: Vec<TodoDonePlotPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LinearTrend {
    slope: f64,
    intercept: f64,
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

    let cache = history_cache::update(root)?;
    let mut done_points = completion_trend_points_for_velocity(&cache.entries, since_secs);

    let mut completed_weight = 0.0;
    let mut comparable_pairs = 0usize;
    let mut completion_events = 0usize;
    for (_old, new) in cache.entries.iter().zip(cache.entries.iter().skip(1)) {
        if new.commit_timestamp < since_secs {
            continue;
        }
        comparable_pairs += 1;
        completed_weight += new.completed_weight_from_previous;
        completion_events += new.completion_events_from_previous;
    }
    if let Some((worktree_completion_delta, worktree_events, latest_commit_ts)) =
        worktree_completion_delta(root, &cache.entries)
    {
        if done_points.is_empty() {
            done_points.push((latest_commit_ts.max(since_secs), 0.0));
        }
        let cumulative =
            done_points.last().map(|(_, y)| *y).unwrap_or(0.0) + worktree_completion_delta;
        done_points.push((now_secs, cumulative));
        comparable_pairs += 1;
        completed_weight += worktree_completion_delta;
        completion_events += worktree_events;
    }

    let trend = linear_trend_by_timestamp(&done_points)?;
    let span_days = (done_points.last()?.0 - done_points.first()?.0) as f64 / SECONDS_PER_DAY;
    if span_days <= 0.0 {
        return None;
    }

    Some(VelocityEstimate {
        weight_per_day: trend.slope * SECONDS_PER_DAY,
        completed_weight,
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
        let (total_weight, done_weight, total_count, done_count, found_milestone) =
            weights_until_milestone_at_ref(root, &entry.commit_hash, &milestone_name);
        if !found_milestone {
            continue;
        }
        points.push(TodoDonePlotPoint {
            date: entry.commit_date.clone(),
            total_weight,
            done_weight,
            total_count,
            done_count,
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

pub fn render_todo_done_plot(plot: &TodoDonePlot, fit: bool) -> String {
    let sampled = downsample_plot_points(&plot.points, 96);
    let today_unix_days = unix_days_from_unix_seconds(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs() as i64),
    );
    let geometry = compute_plot_geometry(&sampled, today_unix_days);
    let total_trend = linear_trend(
        &geometry
            .x_values
            .iter()
            .zip(sampled.iter())
            .map(|(x, p)| (*x, p.total_weight))
            .collect::<Vec<_>>(),
    );
    let done_trend = linear_trend(
        &geometry
            .x_values
            .iter()
            .zip(sampled.iter())
            .map(|(x, p)| (*x, p.done_weight))
            .collect::<Vec<_>>(),
    );

    let mut out = String::new();
    out.push_str("\n");
    out.push_str(&format!("Milestone: {}\n", plot.milestone_name));
    out.push_str("\n");
    out.push_str(&render_textplots_chart(
        &sampled,
        &geometry,
        total_trend,
        done_trend,
        fit,
    ));
    out.push_str(&render_plot_legend());
    if let Some(latest) = plot.points.last() {
        out.push_str("\n");
        out.push_str(&render_plot_stats(latest));
    }
    out.push_str("\n");
    out
}

fn render_plot_legend() -> String {
    let red = ansi_rgb_sample(255, 0, 0);
    let green = ansi_rgb_sample(0, 255, 0);
    let yellow = ansi_rgb_sample(255, 255, 0);
    let cyan = ansi_rgb_sample(0, 255, 255);
    let white = ansi_rgb_sample(255, 255, 255);
    format!(
        "{red} total          {green} done\n{yellow} total trend    {cyan} done trend\n{white} today\n"
    )
}

fn render_plot_stats(latest: &TodoDonePlotPoint) -> String {
    format!(
        "total:  {} tasks  (weight {:.2})\ndone:   {} tasks  (weight {:.2})\n",
        latest.total_count, latest.total_weight, latest.done_count, latest.done_weight,
    )
}

fn ansi_rgb_sample(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{r};{g};{b}m....\x1b[0m")
}

fn render_textplots_chart(
    points: &[TodoDonePlotPoint],
    geometry: &PlotGeometry,
    total_trend: Option<LinearTrend>,
    done_trend: Option<LinearTrend>,
    fit: bool,
) -> String {
    let total_series: Vec<(f32, f32)> = points
        .iter()
        .zip(geometry.x_values.iter())
        .map(|(p, x)| (*x as f32, p.total_weight as f32))
        .collect();
    let done_series: Vec<(f32, f32)> = points
        .iter()
        .zip(geometry.x_values.iter())
        .map(|(p, x)| (*x as f32, p.done_weight as f32))
        .collect();
    let total_trend_series = total_trend
        .map(|t| {
            vec![
                (0.0_f32, t.intercept as f32),
                (
                    geometry.trend_end_x as f32,
                    (t.slope * geometry.trend_end_x + t.intercept) as f32,
                ),
            ]
        })
        .unwrap_or_default();
    let done_trend_series = done_trend
        .map(|t| {
            vec![
                (0.0_f32, t.intercept as f32),
                (
                    geometry.trend_end_x as f32,
                    (t.slope * geometry.trend_end_x + t.intercept) as f32,
                ),
            ]
        })
        .unwrap_or_default();
    let xmax = geometry.chart_x_max as f32;
    let data_ymax: f64 = points
        .iter()
        .map(|p| p.total_weight.max(p.done_weight))
        .fold(0.0, f64::max);
    let mut full_ymax = data_ymax;
    if let Some(t) = total_trend {
        full_ymax = full_ymax
            .max(t.intercept)
            .max(t.slope * geometry.trend_end_x + t.intercept);
    }
    if let Some(t) = done_trend {
        full_ymax = full_ymax
            .max(t.intercept)
            .max(t.slope * geometry.trend_end_x + t.intercept);
    }
    let ymax = (if fit { full_ymax } else { data_ymax }).max(1.0) as f32;
    let today_series = vec![
        (geometry.today_x as f32, 0.0_f32),
        (geometry.today_x as f32, ymax),
    ];

    let total_line_shape = Shape::Lines(&total_series);
    let done_line_shape = Shape::Lines(&done_series);
    let total_point_shape = Shape::Points(&total_series);
    let done_point_shape = Shape::Points(&done_series);
    let total_trend_shape = Shape::Lines(&total_trend_series);
    let done_trend_shape = Shape::Lines(&done_trend_series);
    let today_shape = Shape::Lines(&today_series);
    // Keep a 3:2 canvas (width:height).
    let mut chart = if fit {
        Chart::new(120, 80, 0.0, xmax)
    } else {
        Chart::new_with_y_range(120, 80, 0.0, xmax, 0.0, ymax)
    };
    let mut chart_ref = &mut chart;
    chart_ref = chart_ref.y_label_format(LabelFormat::None);
    if let Some((start_label, end_label)) = x_axis_date_labels(points, geometry) {
        let split_x = xmax / 2.0;
        chart_ref = chart_ref.x_label_format(LabelFormat::Custom(Box::new(move |x| {
            if x <= split_x {
                start_label.clone()
            } else {
                end_label.clone()
            }
        })));
    }
    if !total_trend_series.is_empty() {
        chart_ref = chart_ref.linecolorplot(&total_trend_shape, RGB8::new(255, 255, 0));
    }
    if !done_trend_series.is_empty() {
        chart_ref = chart_ref.linecolorplot(&done_trend_shape, RGB8::new(0, 255, 255));
    }
    chart_ref = chart_ref.linecolorplot(&today_shape, RGB8::new(255, 255, 255));
    chart_ref = chart_ref
        .linecolorplot(&total_line_shape, RGB8::new(255, 0, 0))
        .linecolorplot(&done_line_shape, RGB8::new(0, 255, 0))
        .linecolorplot(&total_point_shape, RGB8::new(255, 0, 0))
        .linecolorplot(&done_point_shape, RGB8::new(0, 255, 0));
    chart_ref.axis();
    chart_ref.figures();
    format!("{chart_ref}\n")
}

fn x_axis_date_labels(
    points: &[TodoDonePlotPoint],
    geometry: &PlotGeometry,
) -> Option<(String, String)> {
    let first_point = points.first()?;
    let first_unix_days = parse_yyyy_mm_dd_to_unix_days(&first_point.date)?;
    let chart_end_days = first_unix_days + geometry.chart_x_max.ceil() as i64;
    let end_date = format_yyyy_mm_dd_from_unix_days(chart_end_days);
    Some((first_point.date.clone(), end_date))
}

#[derive(Debug, Clone, PartialEq)]
struct PlotGeometry {
    x_values: Vec<f64>,
    trend_end_x: f64,
    today_x: f64,
    chart_x_max: f64,
}

fn compute_plot_geometry(
    points: &[TodoDonePlotPoint],
    today_unix_days: Option<i64>,
) -> PlotGeometry {
    let index_fallback = || {
        let x_values: Vec<f64> = (0..points.len()).map(|i| i as f64).collect();
        let start_x = *x_values.first().unwrap_or(&0.0);
        let end_x = *x_values.last().unwrap_or(&0.0);
        let measurement_range = (end_x - start_x).max(0.0);
        let trend_end_x = end_x + (measurement_range / 3.0);
        let today_x = end_x;
        let chart_x_max = trend_end_x.max(today_x).max(1.0);
        PlotGeometry {
            x_values,
            trend_end_x,
            today_x,
            chart_x_max,
        }
    };
    let Some(first_date_days) = points
        .first()
        .and_then(|p| parse_yyyy_mm_dd_to_unix_days(&p.date))
    else {
        return index_fallback();
    };

    let mut x_values = Vec::with_capacity(points.len());
    for point in points {
        let Some(unix_days) = parse_yyyy_mm_dd_to_unix_days(&point.date) else {
            return index_fallback();
        };
        x_values.push((unix_days - first_date_days) as f64);
    }

    let start_x = *x_values.first().unwrap_or(&0.0);
    let end_x = *x_values.last().unwrap_or(&0.0);
    let measurement_range = (end_x - start_x).max(0.0);
    let trend_end_x = end_x + (measurement_range / 3.0);
    let today_x = today_unix_days
        .map(|d| (d - first_date_days) as f64)
        .unwrap_or(end_x);
    let chart_x_max = trend_end_x.max(today_x).max(1.0);
    PlotGeometry {
        x_values,
        trend_end_x,
        today_x,
        chart_x_max,
    }
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

fn completion_trend_points_for_velocity(
    entries: &[history_cache::HistoryCacheEntry],
    since_secs: i64,
) -> Vec<(i64, f64)> {
    let mut points = Vec::new();
    let mut cumulative = 0.0;
    for (old, new) in entries.iter().zip(entries.iter().skip(1)) {
        if new.commit_timestamp < since_secs {
            continue;
        }
        if points.is_empty() {
            points.push((old.commit_timestamp.max(since_secs), cumulative));
        }
        cumulative += new.completed_weight_from_previous;
        points.push((new.commit_timestamp, cumulative));
    }
    points
}

fn worktree_completion_delta(
    root: &Path,
    entries: &[history_cache::HistoryCacheEntry],
) -> Option<(f64, usize, i64)> {
    let latest_entry = entries.last()?;
    let latest_commit_sha = &latest_entry.commit_hash;

    let mut worktree_changed = false;
    let mut completion_delta = 0.0;
    let mut completion_events = 0usize;

    for path in find_task_files(root) {
        let Some(latest_content) = git::file_content_at_ref(root, latest_commit_sha, &path) else {
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
        completion_delta += delta_weight;
        completion_events += delta_events;
    }

    if !worktree_changed {
        return None;
    }

    Some((
        completion_delta,
        completion_events,
        latest_entry.commit_timestamp,
    ))
}

fn linear_trend_by_timestamp(points: &[(i64, f64)]) -> Option<LinearTrend> {
    if points.len() < 2 {
        return None;
    }
    let x0 = points[0].0 as f64;
    let normalized: Vec<(f64, f64)> = points.iter().map(|(ts, y)| (*ts as f64 - x0, *y)).collect();
    linear_trend(&normalized)
}

fn linear_trend(points: &[(f64, f64)]) -> Option<LinearTrend> {
    if points.len() < 2 {
        return None;
    }
    let n = points.len() as f64;
    let mean_x = points.iter().map(|(x, _)| *x).sum::<f64>() / n;
    let mean_y = points.iter().map(|(_, y)| *y).sum::<f64>() / n;
    let mut cov = 0.0;
    let mut var = 0.0;
    for (x, y) in points {
        cov += (x - mean_x) * (y - mean_y);
        var += (x - mean_x) * (x - mean_x);
    }
    if var <= f64::EPSILON {
        return None;
    }
    let slope = cov / var;
    let intercept = mean_y - slope * mean_x;
    Some(LinearTrend { slope, intercept })
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
) -> (f64, f64, usize, usize, bool) {
    let mut paths = git::task_files_at_ref(root, git_ref);
    paths.sort();

    let mut total_weight = 0.0;
    let mut done_weight = 0.0;
    let mut total_count = 0usize;
    let mut done_count = 0usize;
    for path in paths {
        let Some(content) = git::file_content_at_ref(root, git_ref, &path) else {
            continue;
        };
        let items = parser::parse(&content, path);
        for item in items {
            match item {
                FileItem::Task(task) => {
                    let weight = task_total_weight(&task);
                    let count = task_total_count(&task);
                    total_weight += weight;
                    total_count += count;
                    match task.status {
                        Status::Done | Status::Cancelled => {
                            done_weight += weight;
                            done_count += task_done_count(&task);
                        }
                        Status::Todo => {
                            done_count += task_done_count(&task);
                        }
                    }
                }
                FileItem::Milestone(m) => {
                    if m.name == target_milestone {
                        return (total_weight, done_weight, total_count, done_count, true);
                    }
                }
            }
        }
    }

    (0.0, 0.0, 0, 0, false)
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

fn task_total_count(task: &parser::Task) -> usize {
    1 + subtasks_total_count(&task.children)
}

fn subtasks_total_count(children: &[parser::Subtask]) -> usize {
    children
        .iter()
        .map(|c| 1 + subtasks_total_count(&c.children))
        .sum()
}

fn task_done_count(task: &parser::Task) -> usize {
    let self_done = matches!(task.status, Status::Done | Status::Cancelled) as usize;
    self_done + subtasks_done_count(&task.children)
}

fn subtasks_done_count(children: &[parser::Subtask]) -> usize {
    children
        .iter()
        .map(|c| {
            let self_done = matches!(c.status, Status::Done | Status::Cancelled) as usize;
            self_done + subtasks_done_count(&c.children)
        })
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

fn unix_days_from_unix_seconds(unix_seconds: Option<i64>) -> Option<i64> {
    unix_seconds.map(|s| s.div_euclid(86_400))
}

fn parse_yyyy_mm_dd_to_unix_days(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let year = parts.next()?.parse::<i64>().ok()?;
    let month = parts.next()?.parse::<i64>().ok()?;
    let day = parts.next()?.parse::<i64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let yoe = adjusted_year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * month_prime + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn format_yyyy_mm_dd_from_unix_days(unix_days: i64) -> String {
    let (year, month, day) = civil_from_days(unix_days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(unix_days: i64) -> (i64, i64, i64) {
    let z = unix_days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
