//! Builds and prepares [`TodoDonePlot`] data: assembling the todo/done
//! timeline for a milestone from git history plus the live worktree, and
//! the rendering-only geometry/sampling every chart backend draws from
//! (downsampling for display, x-axis mapping, y-axis range, axis date
//! labels). None of this is trend/ETA math — see `trend.rs` for that, and
//! `chart_trends.rs` for how the two combine only once a chart is actually
//! rendered.

use super::date_utils::today_date;
use super::trend::{LinearTrend, date_x_values};
use super::trend_geometry::trend_line_endpoints;
use super::velocity::{require_git_repo, weight_for_depth};
use crate::cli::common::find_task_files;
use crate::lifecycle_cache;
use crate::parser::{self, FileItem, Status};
use chrono::NaiveDate;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct TodoDonePlotPoint {
    pub date: NaiveDate,
    pub total_weight_wt: f64,
    pub done_weight_wt: f64,
    pub total_count_t: usize,
    pub done_count_t: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TodoDonePlot {
    pub milestone_name: String,
    pub points: Vec<TodoDonePlotPoint>,
}

pub fn build_todo_done_plot(root: &Path, milestone_rank: usize) -> Result<TodoDonePlot, String> {
    require_git_repo(root)?;
    if milestone_rank == 0 {
        return Err("milestone rank must be >= 1".to_string());
    }
    let milestone_name = super::report::milestone_name_for_rank(root, milestone_rank)
        .ok_or_else(|| format!("milestone rank {milestone_rank} does not exist"))?;

    let cache = lifecycle_cache::update(root)
        .ok_or_else(|| "no commit history available to build a plot".to_string())?;

    // The milestone's rank (position of the top-level task just before it)
    // is treated as fixed for the whole plot — we use its current, cached
    // rank rather than replaying the milestone's own rank history.
    let target_rank = cache
        .milestones
        .values()
        .find(|m| m.name == milestone_name)
        .map(|m| m.last_known_rank)
        .ok_or_else(|| {
            format!(
                "milestone '{milestone_name}' has not been committed yet; commit it before plotting"
            )
        })?;

    let mut commits = crate::git::commits(root);
    commits.reverse(); // oldest -> newest, matching cache.commit_chain

    let mut points = lifecycle_cache::todo_done_timeline(&cache, &commits, target_rank);
    points.push(worktree_plot_point(root, target_rank));

    Ok(TodoDonePlot {
        milestone_name,
        points,
    })
}

/// Computes the "right now" plot point directly from the on-disk worktree
/// (which may include uncommitted edits), using the same fixed milestone
/// rank as the rest of the timeline.
fn worktree_plot_point(root: &Path, target_rank: Option<usize>) -> TodoDonePlotPoint {
    let today = today_date()
        .unwrap_or(NaiveDate::from_ymd_opt(1970, 1, 1).expect("1970-01-01 is a valid date"));

    let Some(target_rank) = target_rank else {
        // Milestone precedes every task: nothing is ever in scope.
        return TodoDonePlotPoint {
            date: today,
            total_weight_wt: 0.0,
            done_weight_wt: 0.0,
            total_count_t: 0,
            done_count_t: 0,
        };
    };

    let mut total_weight_wt = 0.0;
    let mut done_weight_wt = 0.0;
    let mut total_count_t = 0usize;
    let mut done_count_t = 0usize;
    let mut rank = 0usize;
    for path in find_task_files(root) {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let items = parser::parse(&content, path);
        for item in items {
            let FileItem::Task(task) = item else {
                continue;
            };
            rank += 1;
            if rank > target_rank {
                continue;
            }
            total_weight_wt += 1.0;
            total_count_t += 1;
            if matches!(task.status, Status::Done | Status::Cancelled) {
                done_weight_wt += 1.0;
                done_count_t += 1;
            }
            accumulate_subtasks(
                &task.children,
                2,
                &mut total_weight_wt,
                &mut total_count_t,
                &mut done_weight_wt,
                &mut done_count_t,
            );
        }
    }

    TodoDonePlotPoint {
        date: today,
        total_weight_wt,
        done_weight_wt,
        total_count_t,
        done_count_t,
    }
}

fn accumulate_subtasks(
    children: &[parser::Subtask],
    depth: usize,
    total_weight_wt: &mut f64,
    total_count_t: &mut usize,
    done_weight_wt: &mut f64,
    done_count_t: &mut usize,
) {
    for child in children {
        let w = weight_for_depth(depth);
        *total_weight_wt += w;
        *total_count_t += 1;
        if matches!(child.status, Status::Done | Status::Cancelled) {
            *done_weight_wt += w;
            *done_count_t += 1;
        }
        accumulate_subtasks(
            &child.children,
            depth + 1,
            total_weight_wt,
            total_count_t,
            done_weight_wt,
            done_count_t,
        );
    }
}

/// Purely a chart's x-axis geometry: computed from whatever point series is
/// actually being drawn (typically [`downsample_plot_points`]'s output, not
/// the milestone's full history — see `chart_trends.rs`). Carries no trend
/// math itself; [`compute_plot_y_range`] combines it with fitted trend
/// lines only to answer a rendering question (the y-axis range to draw).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlotGeometry {
    pub(super) x_values: Vec<f64>,
    pub(super) trend_end_x: f64,
    pub(super) today_x: f64,
    pub(super) chart_x_max: f64,
    // Real calendar date (as unix days) that x = 0 maps to. `None` when the
    // points don't carry parseable dates (e.g. in tests), in which case x
    // values are plain indices and can't be converted back to a calendar ETA.
    pub(super) anchor_unix_days: Option<i64>,
}

pub(super) fn compute_plot_geometry(
    points: &[TodoDonePlotPoint],
    today_unix_days: Option<i64>,
) -> PlotGeometry {
    let (x_values, anchor_unix_days) = date_x_values(points);
    let start_x = *x_values.first().unwrap_or(&0.0);
    let end_x = *x_values.last().unwrap_or(&0.0);
    let measurement_range = (end_x - start_x).max(0.0);
    let trend_end_x = end_x + (measurement_range / 3.0);
    let today_x = match anchor_unix_days {
        Some(first_date_days) => today_unix_days
            .map(|d| (d - first_date_days) as f64)
            .unwrap_or(end_x),
        None => end_x,
    };
    let chart_x_max = trend_end_x.max(today_x).max(1.0);
    PlotGeometry {
        x_values,
        trend_end_x,
        today_x,
        chart_x_max,
        anchor_unix_days,
    }
}

/// Maximum number of points a chart draws directly; the milestone's full
/// history (however long) is downsampled to this many points purely for
/// display — trend fitting itself (see [`super::trend::compute_milestone_trends`])
/// always uses the full, undownsampled history.
pub(super) const MAX_CHART_POINTS: usize = 96;

pub(super) fn downsample_plot_points(
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

/// Computes the y-axis range (raw weight, `_wt`) shared by every chart
/// renderer: tight to the data (and both trend lines) when `fit` is set,
/// or starting at zero otherwise. Kept as one function so the different
/// chart backends can't drift apart on how they pick axis bounds.
pub(super) fn compute_plot_y_range(
    points: &[TodoDonePlotPoint],
    geometry: &PlotGeometry,
    total_trend: Option<LinearTrend>,
    done_trend: Option<LinearTrend>,
    fit: bool,
) -> (f64, f64) {
    let data_ymin: f64 = points
        .iter()
        .map(|p| p.done_weight_wt.min(p.total_weight_wt))
        .fold(f64::INFINITY, f64::min);
    let data_ymin = if data_ymin.is_infinite() {
        0.0
    } else {
        data_ymin
    };
    let data_ymax: f64 = points
        .iter()
        .map(|p| p.total_weight_wt.max(p.done_weight_wt))
        .fold(0.0, f64::max);
    let mut full_ymax = data_ymax;
    if let Some(t) = total_trend {
        let e = trend_line_endpoints(t, geometry.trend_end_x);
        full_ymax = full_ymax.max(e.y0).max(e.y1);
    }
    if let Some(t) = done_trend {
        let e = trend_line_endpoints(t, geometry.trend_end_x);
        full_ymax = full_ymax.max(e.y0).max(e.y1);
    }
    let range = if fit {
        (data_ymin, full_ymax.max(data_ymin + 1.0))
    } else {
        (0.0, data_ymax.max(1.0))
    };
    log::debug!(
        "compute_plot_y_range: {} points, data_ymin={data_ymin:.3} data_ymax={data_ymax:.3} full_ymax(incl. trend projections)={full_ymax:.3} fit={fit} -> range={range:?}",
        points.len()
    );
    range
}

pub(super) fn x_axis_date_labels(
    points: &[TodoDonePlotPoint],
    geometry: &PlotGeometry,
) -> Option<(String, String)> {
    let first_point = points.first()?;
    let chart_end_date = first_point
        .date
        .checked_add_signed(chrono::Duration::days(geometry.chart_x_max.ceil() as i64))?;
    Some((first_point.date.to_string(), chart_end_date.to_string()))
}
