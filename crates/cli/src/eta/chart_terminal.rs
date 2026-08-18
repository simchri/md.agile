//! Terminal chart output: `agile when --plot`'s default view, rendered
//! either as a Braille sub-pixel chart (via `textplots`, in
//! [`chart_terminal_braille`]) or, with `--ascii`, as a plain 7-bit-ASCII
//! character grid (in [`chart_terminal_ascii`]).

mod chart_terminal_ascii;
mod chart_terminal_braille;

use super::chart_common::{render_plot_legend, render_plot_stats, render_trend_equations};
use super::chart_trends::PlotData;
use super::eta_math::compute_eta;
use super::eta_text::render_eta_text;
use super::plot_data::{
    MAX_CHART_POINTS, TodoDonePlot, compute_plot_geometry, downsample_plot_points,
};
use super::trend::compute_milestone_trends;
use chart_terminal_ascii::render_ascii_chart;
use chart_terminal_braille::render_textplots_chart;

/// Canvas size shared by both chart backends, expressed in terminal
/// character cells (columns x rows). [`chart_terminal_ascii::render_ascii_chart`]
/// draws one glyph per cell directly at this size.
/// [`chart_terminal_braille::render_textplots_chart`]'s Braille-based canvas
/// packs a 2x4 sub-pixel grid into every terminal cell, so it multiplies
/// this size up into pixels to occupy the same on-screen footprint at
/// higher resolution.
pub(super) const CHART_CHAR_WIDTH: usize = 60;
pub(super) const CHART_CHAR_HEIGHT: usize = 20;

/// Clips the segment `(x0, y0)-(x1, y1)` against the axis-aligned
/// rectangle `[xmin, xmax] x [ymin, ymax]` using the Liang–Barsky
/// algorithm, returning the (possibly shortened) segment that lies inside
/// it, or `None` if the segment doesn't intersect the rectangle at all.
/// Shared by both chart backends: each must clip trend-line/data segments
/// itself, in data space, before handing them to its own pixel/canvas
/// drawing code — the ASCII backend's own row/col conversion and the
/// `textplots` crate's `Scale::linear` both clamp *already-converted*
/// pixel coordinates independently per endpoint rather than clipping the
/// line itself, which distorts a segment's slope whenever one endpoint
/// lies outside the visible range (as can happen for trend lines
/// extending past the y-axis's visible range).
pub(super) fn clip_line_to_rect(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
) -> Option<(f64, f64, f64, f64)> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let mut t0 = 0.0_f64;
    let mut t1 = 1.0_f64;
    // Each (p, q) pair tests one of the rectangle's four boundaries.
    let checks = [
        (-dx, x0 - xmin),
        (dx, xmax - x0),
        (-dy, y0 - ymin),
        (dy, ymax - y0),
    ];
    for (p, q) in checks {
        if p == 0.0 {
            // Parallel to this boundary: reject if outside it.
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                if r > t1 {
                    return None;
                }
                t0 = t0.max(r);
            } else {
                if r < t0 {
                    return None;
                }
                t1 = t1.min(r);
            }
        }
    }
    if t0 > t1 {
        return None;
    }
    Some((x0 + t0 * dx, y0 + t0 * dy, x0 + t1 * dx, y0 + t1 * dy))
}

pub fn render_todo_done_plot(plot: &TodoDonePlot, extra: f64, ascii: bool, color: bool) -> String {
    let today_unix_days = super::date_utils::today_unix_days();
    let (total_trend, done_trend) = compute_milestone_trends(plot);
    log::debug!("render_todo_done_plot: total_trend = {:?}", total_trend);
    log::debug!("render_todo_done_plot: done_trend = {:?}", done_trend);

    let mut out = String::new();
    out.push_str("\n");
    out.push_str(&format!("Milestone: {}\n", plot.milestone_name));
    out.push_str("\n");
    if ascii {
        // The ASCII backend's fixed-size character grid (see
        // `chart_terminal_ascii`) can't usefully show more distinct data
        // points than it has columns, so its series is downsampled for
        // display.
        let sampled = downsample_plot_points(&plot.points, MAX_CHART_POINTS);
        let geometry = compute_plot_geometry(&sampled, today_unix_days, extra);
        let plot_data = PlotData {
            sampled,
            geometry,
            total_trend,
            done_trend,
        };
        out.push_str(&render_ascii_chart(&plot_data, color));
    } else {
        // `textplots` draws straight lines directly between whatever
        // points it's given (see `chart_terminal_braille`), so the Braille
        // chart plots the milestone's full point history unsampled.
        let geometry = compute_plot_geometry(&plot.points, today_unix_days, extra);
        let plot_data = PlotData {
            sampled: plot.points.clone(),
            geometry,
            total_trend,
            done_trend,
        };
        out.push_str(&render_textplots_chart(&plot_data, color));
    }
    out.push_str(&render_plot_legend(ascii, color));
    out.push_str("\n");
    out.push_str(&render_trend_equations(total_trend, done_trend, color));
    if let Some(latest) = plot.points.last() {
        out.push_str("\n");
        out.push_str(&render_plot_stats(latest));
    }
    out.push_str("\n");
    out.push_str(&render_eta_text(
        compute_eta(total_trend, done_trend, today_unix_days),
        today_unix_days,
    ));
    out
}
