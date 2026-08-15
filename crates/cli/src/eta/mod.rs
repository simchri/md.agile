//! ETA/velocity computation primitives.

use crate::cli::common::find_task_files;
use crate::git;
use crate::lifecycle_cache;
use crate::parser::{self, FileItem, Status};
use rgb::RGB8;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use textplots::{Chart, ColorPlot, LabelBuilder, LabelFormat, Shape};

/// Number of days in a week, used to convert day-based rates (`_wtpd`) to
/// week-based rates (`_wtpw`) for display purposes.
const DAYS_PER_WEEK: f64 = 7.0;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityEstimate {
    /// Slope of the whole-project done-weight trend line, in weight/week:
    /// how fast completed weight is accumulating.
    pub velocity_wtpw: Option<f64>,
    /// Slope of the whole-project total-weight trend line, in weight/week:
    /// how fast the backlog itself is growing (new/expanded work).
    pub creep_wtpw: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TodoDonePlotPoint {
    pub date: String,
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct LinearTrend {
    slope_wtpd: f64,
    intercept_wt: f64,
}

/// The estimated time of arrival at a milestone: the calendar date (as unix
/// days) where the total and done trend lines intersect.
#[derive(Debug, Clone, Copy, PartialEq)]
struct EtaEstimate {
    unix_days: i64,
}

/// Estimates current velocity and creep (see [`VelocityEstimate`]), scoped
/// to the next milestone (rank 1) — exactly the same trend lines
/// [`build_todo_done_plot`]/`agile when --plot` would fit and show for that
/// milestone.
///
/// Returns an error when `root` isn't a git repository, or when there's no
/// future milestone to scope to (see [`build_todo_done_plot`]).
pub fn estimate_velocity(root: &Path) -> Result<VelocityEstimate, String> {
    estimate_velocity_with_window(root, 1, None)
}

/// Like [`estimate_velocity`], but scoped to a caller-provided milestone
/// rank (see [`build_todo_done_plot`]) and, optionally, further restricted
/// to a caller-provided trailing window (in days) — matching the
/// `--next`/`--last` flags on `agile when --velocity`. `window_days: None`
/// applies no day-based restriction, using the milestone's whole history.
///
/// Velocity and creep are the slopes of the exact same done-weight/
/// total-weight trend lines `agile when --plot` fits and draws for the same
/// milestone (see [`compute_plot_trends`]) — both expressed in weight/week.
/// Either metric independently resolves to `None` when its trend line can't
/// be computed (fewer than two distinct points in the window).
pub fn estimate_velocity_with_window(
    root: &Path,
    milestone_rank: usize,
    window_days: Option<u32>,
) -> Result<VelocityEstimate, String> {
    require_git_repo(root)?;
    if window_days == Some(0) {
        return Ok(VelocityEstimate {
            velocity_wtpw: None,
            creep_wtpw: None,
        });
    }

    let mut plot = build_todo_done_plot(root, milestone_rank)?;

    let today = today_unix_days();
    if let Some(cutoff) = window_days.and_then(|days| today.map(|t| t - i64::from(days))) {
        plot.points
            .retain(|p| parse_yyyy_mm_dd_to_unix_days(&p.date).is_none_or(|d| d >= cutoff));
    }

    let trends = compute_plot_trends(&plot, today);

    Ok(VelocityEstimate {
        velocity_wtpw: trends.done_trend.map(|t| t.slope_wtpd * DAYS_PER_WEEK),
        creep_wtpw: trends.total_trend.map(|t| t.slope_wtpd * DAYS_PER_WEEK),
    })
}

/// Ensures `root` is a git repository, returning a consistent error message
/// (shared by every `agile when` entry point that needs commit history).
fn require_git_repo(root: &Path) -> Result<(), String> {
    if !git::is_git_repo(root) {
        return Err("`agile when` requires a git repository".to_string());
    }
    Ok(())
}

pub fn build_todo_done_plot(root: &Path, milestone_rank: usize) -> Result<TodoDonePlot, String> {
    require_git_repo(root)?;
    if milestone_rank == 0 {
        return Err("milestone rank must be >= 1".to_string());
    }
    let milestone_name = milestone_name_for_rank(root, milestone_rank)
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

    let mut commits = git::commits(root);
    commits.reverse(); // oldest -> newest, matching cache.commit_chain

    let mut points = lifecycle_cache::todo_done_timeline(&cache, &commits, target_rank);
    points.push(worktree_plot_point(root, target_rank));

    Ok(TodoDonePlot {
        milestone_name,
        points,
    })
}

/// Builds the bare `agile when` report: the ETA time span for every future
/// milestone (see [`future_milestone_names`]), one per line, in backlog
/// order — matching README.vision.md's list-mode output. A milestone whose
/// ETA can't be computed (e.g. not committed yet, or no convergent trend)
/// shows "unknown" instead of a span.
pub fn build_when_report(root: &Path) -> Result<String, String> {
    require_git_repo(root)?;
    let today = today_unix_days();
    let mut out = String::new();
    for (index, name) in future_milestone_names(root).into_iter().enumerate() {
        let rank = index + 1;
        let eta = build_todo_done_plot(root, rank)
            .ok()
            .and_then(|plot| eta_for_plot(&plot, today));
        out.push_str(&render_when_line(&name, eta, today));
    }
    Ok(out)
}

/// Computes the "right now" plot point directly from the on-disk worktree
/// (which may include uncommitted edits), using the same fixed milestone
/// rank as the rest of the timeline.
fn worktree_plot_point(root: &Path, target_rank: Option<usize>) -> TodoDonePlotPoint {
    let today = format_yyyy_mm_dd_from_unix_days(today_unix_days().unwrap_or(0));

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

/// Returns today's date as unix days (days since the unix epoch), or `None`
/// if the system clock is unavailable/invalid.
fn today_unix_days() -> Option<i64> {
    unix_days_from_unix_seconds(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs() as i64),
    )
}

/// The trend lines and supporting geometry/sampling used both to render the
/// terminal chart and to compute the milestone's ETA.
struct PlotTrends {
    sampled: Vec<TodoDonePlotPoint>,
    geometry: PlotGeometry,
    total_trend: Option<LinearTrend>,
    done_trend: Option<LinearTrend>,
}

fn compute_plot_trends(plot: &TodoDonePlot, today_unix_days: Option<i64>) -> PlotTrends {
    let sampled = downsample_plot_points(&plot.points, 96);
    let geometry = compute_plot_geometry(&sampled, today_unix_days);
    let total_trend = linear_trend(
        &geometry
            .x_values
            .iter()
            .zip(sampled.iter())
            .map(|(x, p)| (*x, p.total_weight_wt))
            .collect::<Vec<_>>(),
    );
    let done_trend = linear_trend(
        &geometry
            .x_values
            .iter()
            .zip(sampled.iter())
            .map(|(x, p)| (*x, p.done_weight_wt))
            .collect::<Vec<_>>(),
    );
    PlotTrends {
        sampled,
        geometry,
        total_trend,
        done_trend,
    }
}

/// Computes a milestone's ETA (see [`compute_eta`]) directly from its plot
/// data, deriving the trend lines the same way the chart does.
fn eta_for_plot(plot: &TodoDonePlot, today_unix_days: Option<i64>) -> Option<EtaEstimate> {
    let trends = compute_plot_trends(plot, today_unix_days);
    compute_eta(
        trends.total_trend,
        trends.done_trend,
        trends.geometry.anchor_unix_days,
        today_unix_days,
    )
}

pub fn render_todo_done_plot(plot: &TodoDonePlot, fit: bool, ascii: bool) -> String {
    let today_unix_days = today_unix_days();
    let trends = compute_plot_trends(plot, today_unix_days);

    let mut out = String::new();
    out.push_str("\n");
    out.push_str(&format!("Milestone: {}\n", plot.milestone_name));
    out.push_str("\n");
    if ascii {
        out.push_str(&render_ascii_chart(
            &trends.sampled,
            &trends.geometry,
            trends.total_trend,
            trends.done_trend,
            fit,
        ));
    } else {
        out.push_str(&render_textplots_chart(
            &trends.sampled,
            &trends.geometry,
            trends.total_trend,
            trends.done_trend,
            fit,
        ));
    }
    out.push_str(&render_plot_legend(ascii));
    out.push_str(&render_trend_equations(
        trends.total_trend,
        trends.done_trend,
        trends.geometry.anchor_unix_days,
    ));
    if let Some(latest) = plot.points.last() {
        out.push_str("\n");
        out.push_str(&render_plot_stats(latest));
    }
    out.push_str("\n");
    let eta = compute_eta(
        trends.total_trend,
        trends.done_trend,
        trends.geometry.anchor_unix_days,
        today_unix_days,
    );
    out.push_str(&render_eta_text(eta, today_unix_days));
    out
}

/// Legend for the plot's four data lines. In `ascii` mode, the fixed
/// symbols used by [`render_ascii_chart`] (`o`/`@`/`O`/`0`/`Q`) are shown
/// alongside their colors, since not every ASCII-only terminal renders
/// ANSI color; in the default mode (Braille chart), color is the only
/// differentiator, matching [`render_textplots_chart`]'s palette.
fn render_plot_legend(ascii: bool) -> String {
    let red = ansi_rgb_sample(255, 0, 0);
    let green = ansi_rgb_sample(0, 255, 0);
    let yellow = ansi_rgb_sample(255, 255, 0);
    let cyan = ansi_rgb_sample(0, 255, 255);
    let white = ansi_rgb_sample(255, 255, 255);
    if ascii {
        format!(
            "{red} o total          {green} @ done\n{yellow} O total trend    {cyan} 0 done trend\n{white} Q today\n"
        )
    } else {
        format!(
            "{red} total          {green} done\n{yellow} total trend    {cyan} done trend\n{white} today\n"
        )
    }
}

/// Renders the fitted total/done trend lines as explicit `y = a + b*x`
/// equations, so the slope (creep/velocity) and intercept (cutoff: the
/// trend's weight at `x = 0`) that drive the chart and the ETA are visible,
/// not just implied by the drawn lines. The slope is shown in weight/week
/// — matching the unit `--velocity` reports — with `x` in weeks since
/// `anchor_unix_days` (or a plain point index when no real dates are
/// available — see [`PlotGeometry::anchor_unix_days`]).
fn render_trend_equations(
    total_trend: Option<LinearTrend>,
    done_trend: Option<LinearTrend>,
    anchor_unix_days: Option<i64>,
) -> String {
    let x_desc = match anchor_unix_days {
        Some(anchor) => format!("weeks since {}", format_yyyy_mm_dd_from_unix_days(anchor)),
        None => "point index".to_string(),
    };
    let yellow = ansi_rgb_text(255, 255, 0, "total");
    let cyan = ansi_rgb_text(0, 255, 255, "done");
    format!(
        "Trend lines (x = {x_desc}):\n  {yellow} = {}\n  {cyan}  = {}\n",
        render_trend_equation(total_trend),
        render_trend_equation(done_trend),
    )
}

/// Renders a single trend line as `<intercept> + <slope>/week * x`, or
/// "unknown" when the trend couldn't be fit (see [`linear_trend`]). The
/// fitted slope is per day (see [`LinearTrend`]); it's converted to
/// weight/week here purely for display, to match `--velocity`'s unit.
fn render_trend_equation(trend: Option<LinearTrend>) -> String {
    match trend {
        Some(t) => format!(
            "{:.2} + {:.2}/week * x",
            t.intercept_wt,
            t.slope_wtpd * DAYS_PER_WEEK
        ),
        None => "unknown".to_string(),
    }
}

fn render_plot_stats(latest: &TodoDonePlotPoint) -> String {
    format!(
        "total:  {} tasks  (weight {:.2})\ndone:   {} tasks  (weight {:.2})\n",
        latest.total_count_t, latest.total_weight_wt, latest.done_count_t, latest.done_weight_wt,
    )
}

/// Renders the raw plot data (task counts and weights, no trend line
/// fitting) as a simple table, one row per point.
pub fn render_todo_done_data(plot: &TodoDonePlot) -> String {
    let mut out = String::new();
    out.push_str(&format!("Milestone: {}\n\n", plot.milestone_name));
    out.push_str(&format!(
        "{:<12}{:>7}{:>7}{:>10}{:>9}\n",
        "Date", "Total", "Done", "Total Wt", "Done Wt"
    ));
    for point in &plot.points {
        out.push_str(&format!(
            "{:<12}{:>7}{:>7}{:>10.2}{:>9.2}\n",
            point.date,
            point.total_count_t,
            point.done_count_t,
            point.total_weight_wt,
            point.done_weight_wt
        ));
    }
    out
}

fn ansi_rgb_sample(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{r};{g};{b}m....\x1b[0m")
}

/// Colors `text` itself (as opposed to [`ansi_rgb_sample`]'s fixed "...."
/// swatch), for labels that need to stay readable (e.g. trend equations).
fn ansi_rgb_text(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m")
}

/// Computes the y-axis range (raw weight, `_wt`) shared by every chart
/// renderer: tight to the data (and both trend lines) when `fit` is set,
/// or starting at zero otherwise. Kept as one function so the different
/// chart backends (see [`render_textplots_chart`], [`render_ascii_chart`])
/// can't drift apart on how they pick axis bounds.
fn compute_plot_y_range(
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
        full_ymax = full_ymax
            .max(t.intercept_wt)
            .max(t.slope_wtpd * geometry.trend_end_x + t.intercept_wt);
    }
    if let Some(t) = done_trend {
        full_ymax = full_ymax
            .max(t.intercept_wt)
            .max(t.slope_wtpd * geometry.trend_end_x + t.intercept_wt);
    }
    if fit {
        (data_ymin, full_ymax.max(data_ymin + 1.0))
    } else {
        (0.0, data_ymax.max(1.0))
    }
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
        .map(|(p, x)| (*x as f32, p.total_weight_wt as f32))
        .collect();
    let done_series: Vec<(f32, f32)> = points
        .iter()
        .zip(geometry.x_values.iter())
        .map(|(p, x)| (*x as f32, p.done_weight_wt as f32))
        .collect();
    let total_trend_series = total_trend
        .map(|t| {
            vec![
                (0.0_f32, t.intercept_wt as f32),
                (
                    geometry.trend_end_x as f32,
                    (t.slope_wtpd * geometry.trend_end_x + t.intercept_wt) as f32,
                ),
            ]
        })
        .unwrap_or_default();
    let done_trend_series = done_trend
        .map(|t| {
            vec![
                (0.0_f32, t.intercept_wt as f32),
                (
                    geometry.trend_end_x as f32,
                    (t.slope_wtpd * geometry.trend_end_x + t.intercept_wt) as f32,
                ),
            ]
        })
        .unwrap_or_default();
    let xmax = geometry.chart_x_max as f32;
    let (ymin, ymax) = compute_plot_y_range(points, geometry, total_trend, done_trend, fit);
    let (ymin, ymax) = (ymin as f32, ymax as f32);
    let today_series = vec![
        (geometry.today_x as f32, ymin),
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
    let mut chart = Chart::new_with_y_range(120, 80, 0.0, xmax, ymin, ymax);
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

/// Fixed pixel-grid size for [`render_ascii_chart`]. Deliberately much
/// coarser than [`render_textplots_chart`]'s Braille-based canvas (which
/// packs a 2x4 sub-pixel grid into every terminal cell) — a plain grid of
/// one character per cell can't reach the same resolution, but works on
/// any 7-bit-ASCII terminal with no Unicode/Braille/ANSI-color support.
const ASCII_CHART_WIDTH: usize = 80;
const ASCII_CHART_HEIGHT: usize = 24;

/// One glyph (plus optional RGB color for terminals that support it) drawn
/// onto the ASCII chart's character grid, in growing draw-order priority:
/// later draws win ties on the same cell.
#[derive(Debug, Clone, Copy)]
struct AsciiGlyph {
    ch: char,
    color: Option<(u8, u8, u8)>,
}

/// A fixed-size character grid with (x in days, y in weight) axis mapping,
/// used to build up [`render_ascii_chart`]'s output one series at a time.
struct AsciiCanvas {
    grid: Vec<Vec<Option<AsciiGlyph>>>,
    width: usize,
    height: usize,
    xmax: f64,
    ymin: f64,
    yspan: f64,
}

impl AsciiCanvas {
    fn new(width: usize, height: usize, xmax: f64, ymin: f64, yspan: f64) -> Self {
        Self {
            grid: vec![vec![None; width]; height],
            width,
            height,
            xmax,
            ymin,
            yspan,
        }
    }

    fn to_col(&self, x: f64) -> usize {
        let frac = (x / self.xmax).clamp(0.0, 1.0);
        ((frac * (self.width - 1) as f64).round() as usize).min(self.width - 1)
    }

    fn to_row(&self, y: f64) -> usize {
        let frac = ((y - self.ymin) / self.yspan).clamp(0.0, 1.0);
        // Row 0 is the top of the grid, so higher y values get lower rows.
        (((1.0 - frac) * (self.height - 1) as f64).round() as usize).min(self.height - 1)
    }

    fn set(&mut self, col: usize, row: usize, ch: char, color: (u8, u8, u8)) {
        self.grid[row][col] = Some(AsciiGlyph {
            ch,
            color: Some(color),
        });
    }

    fn draw_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, ch: char, color: (u8, u8, u8)) {
        let (col0, row0) = (self.to_col(x0), self.to_row(y0));
        let (col1, row1) = (self.to_col(x1), self.to_row(y1));
        let steps = col0.abs_diff(col1).max(row0.abs_diff(row1)).max(1);
        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            let col = (col0 as f64 + (col1 as f64 - col0 as f64) * t).round() as usize;
            let row = (row0 as f64 + (row1 as f64 - row0 as f64) * t).round() as usize;
            self.set(col.min(self.width - 1), row.min(self.height - 1), ch, color);
        }
    }

    fn draw_series(
        &mut self,
        points: &[TodoDonePlotPoint],
        x_values: &[f64],
        ch: char,
        color: (u8, u8, u8),
        value_of: impl Fn(&TodoDonePlotPoint) -> f64,
    ) {
        let mut prev: Option<(f64, f64)> = None;
        for (p, x) in points.iter().zip(x_values.iter()) {
            let y = value_of(p);
            if let Some((px, py)) = prev {
                self.draw_line(px, py, *x, y, ch, color);
            }
            self.set(self.to_col(*x), self.to_row(y), ch, color);
            prev = Some((*x, y));
        }
    }
}

/// Renders the same total/done/trend/today lines [`render_textplots_chart`]
/// draws, but onto a plain fixed-size character grid using only 7-bit ASCII
/// symbols (`o`, `@`, `O`, `0`, `Q`) — one distinct, large/round symbol per
/// series, so the chart stays readable even without ANSI color support.
/// Color is still applied (matching [`render_plot_legend`]'s palette) for
/// terminals that do support it; symbols alone carry the same information
/// otherwise. Resolution is intentionally much lower than the default
/// Braille-based chart: one glyph per terminal cell instead of a packed
/// sub-pixel grid.
fn render_ascii_chart(
    points: &[TodoDonePlotPoint],
    geometry: &PlotGeometry,
    total_trend: Option<LinearTrend>,
    done_trend: Option<LinearTrend>,
    fit: bool,
) -> String {
    let width = ASCII_CHART_WIDTH;
    let height = ASCII_CHART_HEIGHT;
    let xmax = geometry.chart_x_max.max(1.0);
    let (ymin, ymax) = compute_plot_y_range(points, geometry, total_trend, done_trend, fit);
    let yspan = (ymax - ymin).max(1e-9);

    let mut canvas = AsciiCanvas::new(width, height, xmax, ymin, yspan);

    // Today marker (drawn first so data/trend lines stay visible on top of
    // it where they cross).
    let today_col = canvas.to_col(geometry.today_x);
    for row in 0..height {
        canvas.grid[row][today_col].get_or_insert(AsciiGlyph {
            ch: 'Q',
            color: Some((255, 255, 255)),
        });
    }

    // Trend lines (straight two-point lines over the full trend window).
    if let Some(t) = total_trend {
        canvas.draw_line(
            0.0,
            t.intercept_wt,
            geometry.trend_end_x,
            t.slope_wtpd * geometry.trend_end_x + t.intercept_wt,
            'O',
            (255, 255, 0),
        );
    }
    if let Some(t) = done_trend {
        canvas.draw_line(
            0.0,
            t.intercept_wt,
            geometry.trend_end_x,
            t.slope_wtpd * geometry.trend_end_x + t.intercept_wt,
            '0',
            (0, 255, 255),
        );
    }

    // Raw data series (drawn last so they stay on top of trend/today lines).
    canvas.draw_series(points, &geometry.x_values, 'o', (255, 0, 0), |p| {
        p.total_weight_wt
    });
    canvas.draw_series(points, &geometry.x_values, '@', (0, 255, 0), |p| {
        p.done_weight_wt
    });

    let mut out = String::new();
    for row in &canvas.grid {
        for cell in row {
            match cell {
                Some(glyph) => match glyph.color {
                    Some((r, g, b)) => out.push_str(&ansi_rgb_text(r, g, b, &glyph.ch.to_string())),
                    None => out.push(glyph.ch),
                },
                None => out.push(' '),
            }
        }
        out.push('\n');
    }
    if let Some((start_label, end_label)) = x_axis_date_labels(points, geometry) {
        let pad = width.saturating_sub(start_label.len() + end_label.len());
        out.push_str(&start_label);
        out.push_str(&" ".repeat(pad));
        out.push_str(&end_label);
        out.push('\n');
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
struct PlotGeometry {
    x_values: Vec<f64>,
    trend_end_x: f64,
    today_x: f64,
    chart_x_max: f64,
    // Real calendar date (as unix days) that x = 0 maps to. `None` when the
    // points don't carry parseable dates (e.g. in tests), in which case x
    // values are plain indices and can't be converted back to a calendar ETA.
    anchor_unix_days: Option<i64>,
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
            anchor_unix_days: None,
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
        anchor_unix_days: Some(first_date_days),
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
    let slope_wtpd = cov / var;
    let intercept_wt = mean_y - slope_wtpd * mean_x;
    Some(LinearTrend {
        slope_wtpd,
        intercept_wt,
    })
}

/// Computes the ETA to a milestone as the intersection of the total and done
/// trend lines, expressed relative to `anchor_unix_days` (the calendar date
/// that trend-line x = 0 maps to). Returns `None` when either trend line is
/// missing, the lines are parallel (no single intersection), the anchor date
/// couldn't be determined (e.g. no real dates available), or the
/// intersection falls on or before today (already reached, or unknowable).
///
/// This function is purely date/time math — it performs no string
/// formatting; see [`render_eta_text`] for that.
fn compute_eta(
    total_trend: Option<LinearTrend>,
    done_trend: Option<LinearTrend>,
    anchor_unix_days: Option<i64>,
    today_unix_days: Option<i64>,
) -> Option<EtaEstimate> {
    let total = total_trend?;
    let done = done_trend?;
    let anchor = anchor_unix_days?;
    let today = today_unix_days?;

    let slope_diff = total.slope_wtpd - done.slope_wtpd;
    if slope_diff.abs() <= f64::EPSILON {
        return None;
    }
    let x_intersect = (done.intercept_wt - total.intercept_wt) / slope_diff;
    let unix_days = anchor + x_intersect.round() as i64;

    if unix_days <= today {
        return None;
    }

    Some(EtaEstimate { unix_days })
}

/// Formats a day count as a human-friendly time span. Per README.vision.md:
/// days below a week, weeks below 8 weeks, years from 3 years and higher,
/// months otherwise.
fn format_days_as_span(days: i64) -> String {
    const DAYS_PER_WEEK: i64 = 7;
    const DAYS_PER_MONTH: f64 = 30.44;
    const DAYS_PER_YEAR: f64 = 365.25;
    const YEAR_THRESHOLD_DAYS: i64 = 3 * 365;

    if days < DAYS_PER_WEEK {
        return pluralize(days, "day");
    }
    if days < 8 * DAYS_PER_WEEK {
        return pluralize((days as f64 / DAYS_PER_WEEK as f64).round() as i64, "week");
    }
    if days < YEAR_THRESHOLD_DAYS {
        return pluralize((days as f64 / DAYS_PER_MONTH).round() as i64, "month");
    }
    pluralize((days as f64 / DAYS_PER_YEAR).round() as i64, "year")
}

fn pluralize(n: i64, unit: &str) -> String {
    if n == 1 {
        format!("1 {unit}")
    } else {
        format!("{n} {unit}s")
    }
}

/// Resolves an ETA (and "today") down to its human-readable span, or `None`
/// if either half is missing (meaning "unknown" to callers).
fn eta_span(eta: Option<EtaEstimate>, today_unix_days: Option<i64>) -> Option<String> {
    let (eta, today) = (eta?, today_unix_days?);
    Some(format_days_as_span(eta.unix_days - today))
}

/// Renders the "ETA: ..." / "ETA date: ..." text block shown after the plot.
/// All string formatting (date formatting and the day-count-to-span
/// conversion) lives here; [`compute_eta`] only ever deals in dates.
fn render_eta_text(eta: Option<EtaEstimate>, today_unix_days: Option<i64>) -> String {
    let Some(span) = eta_span(eta, today_unix_days) else {
        return format!("{:<10}unknown\n", "ETA:");
    };
    let date = format_yyyy_mm_dd_from_unix_days(eta.unwrap().unix_days);
    format!("{:<10}{span}\n{:<10}{date}\n", "ETA:", "ETA date:")
}

/// Renders the "velocity: ..." / "creep: ..." text block shown for
/// `agile when --velocity`. Labels are left-padded and units/numbers are
/// aligned so the numeric value is always the last column on each line;
/// either line shows "unknown" in place of the number when its trend can't
/// be computed.
pub fn render_velocity_text(estimate: VelocityEstimate) -> String {
    format!(
        "{}\n{}\n",
        render_velocity_line("velocity:", estimate.velocity_wtpw),
        render_velocity_line("creep:", estimate.creep_wtpw),
    )
}

fn render_velocity_line(label: &str, weight_per_week: Option<f64>) -> String {
    match weight_per_week {
        Some(value) => format!("{label:<9} weight/week   {value:.2}"),
        None => format!("{label:<9} unknown"),
    }
}

/// Renders one line of the bare `agile when` report: the ETA span
/// left-padded to line up with the milestone name, matching the column
/// width used by [`render_eta_text`].
fn render_when_line(name: &str, eta: Option<EtaEstimate>, today_unix_days: Option<i64>) -> String {
    let span = eta_span(eta, today_unix_days).unwrap_or_else(|| "unknown".to_string());
    format!("{span:<10}{name}\n")
}

/// Returns the names of all *future* milestones, in backlog order, i.e. the
/// milestones that appear after the first incomplete task in the backlog
/// (matching `agile milestones --list --next`'s semantics). Milestones that
/// only have completed tasks above them have already been reached and are
/// skipped.
fn future_milestone_names(root: &Path) -> Vec<String> {
    let mut milestones = Vec::new();
    let mut seen_incomplete_task = false;
    for path in find_task_files(root) {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let items = parser::parse(&content, path);
        for item in items {
            match item {
                FileItem::Task(task) => {
                    if !seen_incomplete_task && !task_subtree_complete(&task) {
                        seen_incomplete_task = true;
                    }
                }
                FileItem::Milestone(m) => {
                    if seen_incomplete_task {
                        milestones.push(m.name);
                    }
                }
            }
        }
    }
    milestones
}

/// Returns the name of the `milestone_rank`-th *future* milestone (1-based).
/// See [`future_milestone_names`] for what counts as "future".
fn milestone_name_for_rank(root: &Path, milestone_rank: usize) -> Option<String> {
    future_milestone_names(root)
        .into_iter()
        .nth(milestone_rank - 1)
}

fn is_closed_status(status: &Status) -> bool {
    matches!(status, Status::Done | Status::Cancelled)
}

fn task_subtree_complete(task: &parser::Task) -> bool {
    is_closed_status(&task.status) && task.children.iter().all(subtask_subtree_complete)
}

fn subtask_subtree_complete(subtask: &parser::Subtask) -> bool {
    is_closed_status(&subtask.status) && subtask.children.iter().all(subtask_subtree_complete)
}

// Currently unused now that velocity estimation is disabled (history cache removal),
// but retained for reuse once velocity is rebuilt on top of the lifecycle cache.
#[allow(dead_code)]
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

pub(crate) fn weight_for_depth(depth: usize) -> f64 {
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
