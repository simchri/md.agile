//! Builds and prepares [`TodoDonePlot`] data: assembling the todo/done
//! timeline for a milestone from git history plus the live worktree, and
//! the rendering-only geometry/sampling every chart backend draws from
//! (downsampling for display, x-axis mapping, y-axis range, axis date
//! labels). None of this is trend/ETA math — see `trend.rs` for that, and
//! `chart_trends.rs` for how the two combine only once a chart is actually
//! rendered.

use super::date_utils::{date_from_unix_days, today_date, unix_days_from_date};
use super::trend::LinearTrend;
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
///
/// Every `x` field here — like every `x`/`anchor_x_d` used by trend fitting
/// (see `trend.rs`) and ETA math (see `eta_math.rs`) — is expressed in the
/// same single coordinate system throughout the whole graphing pipeline:
/// unix days (whole days since the Unix epoch, 1970-01-01). Nothing here
/// shifts that origin to the plotted series' first point, today, or
/// anywhere else — `chart_x_min`/`chart_x_max` just happen to be large
/// numbers (e.g. ~20000) rather than small ones. Renderers must scale
/// against the actual `[chart_x_min, chart_x_max]` window, not assume it
/// starts at 0.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlotGeometry {
    pub(super) x_values: Vec<f64>,
    pub(super) chart_x_min: f64,
    pub(super) trend_end_x: f64,
    pub(super) today_x: f64,
    pub(super) chart_x_max: f64,
}

/// Default `--extra` factor (see [`compute_plot_geometry`]): how far past
/// the last data point the chart's x-axis (and, in turn, its trend lines)
/// extend when the CLI flag isn't overridden.
pub const DEFAULT_EXTRA: f64 = 1.3;

/// Computes the chart's x-axis geometry: `extra` multiplies the plotted
/// x-range (`start_x` to `end_x`) to produce `trend_end_x`, the chart's
/// right edge past which trend lines are no longer drawn — e.g. `extra =
/// 1.3` extends the chart 30% past the last data point, relative to the
/// full historical span. See `--extra`'s CLI documentation.
pub(super) fn compute_plot_geometry(
    points: &[TodoDonePlotPoint],
    today_unix_days: Option<i64>,
    extra: f64,
) -> PlotGeometry {
    let x_values: Vec<f64> = points
        .iter()
        .map(|point| unix_days_from_date(point.date) as f64)
        .collect();
    let start_x = *x_values.first().unwrap_or(&0.0);
    let end_x = *x_values.last().unwrap_or(&0.0);
    let trend_end_x = start_x + (end_x - start_x) * extra;
    let today_x = today_unix_days.map(|d| d as f64).unwrap_or(end_x);
    let chart_x_max = trend_end_x.max(today_x).max(start_x + 1.0);
    PlotGeometry {
        x_values,
        chart_x_min: start_x,
        trend_end_x,
        today_x,
        chart_x_max,
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
/// renderer: always tight to the data and both trend lines (within the
/// `--extra`-extended x-window), so the chart never clips off a trend
/// line's visible endpoint. Kept as one function so the different chart
/// backends can't drift apart on how they pick axis bounds.
pub(super) fn compute_plot_y_range(
    points: &[TodoDonePlotPoint],
    geometry: &PlotGeometry,
    total_trend: Option<LinearTrend>,
    done_trend: Option<LinearTrend>,
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
    let range = (data_ymin, full_ymax.max(data_ymin + 1.0));
    log::debug!(
        "compute_plot_y_range: {} points, data_ymin={data_ymin:.3} data_ymax={data_ymax:.3} full_ymax(incl. trend projections)={full_ymax:.3} -> range={range:?}",
        points.len()
    );
    range
}

pub(super) fn x_axis_date_labels(
    points: &[TodoDonePlotPoint],
    geometry: &PlotGeometry,
) -> Option<(String, String)> {
    let first_point = points.first()?;
    let chart_end_date = date_from_unix_days(geometry.chart_x_max.round() as i64)?;
    Some((first_point.date.to_string(), chart_end_date.to_string()))
}
