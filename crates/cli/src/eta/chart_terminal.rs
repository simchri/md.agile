//! Terminal chart output: `agile when --plot`'s default view, rendered
//! either as a Braille sub-pixel chart (via `textplots`) or, with
//! `--ascii`, as a plain 7-bit-ASCII character grid.

use super::chart_common::{
    ansi_rgb_text, render_plot_legend, render_plot_stats, render_plot_trend_equations,
};
use super::plot_data::{TodoDonePlot, TodoDonePlotPoint, x_axis_date_labels};
use super::trend::{LinearTrend, PlotTrends, compute_plot_trends, render_eta_text};
use super::trend_geometry::{trend_line_endpoints, trend_line_endpoints_f32};
use rgb::RGB8;
use textplots::{Chart, ColorPlot, LabelBuilder, LabelFormat, Plot, Shape};

pub fn render_todo_done_plot(plot: &TodoDonePlot, fit: bool, ascii: bool, color: bool) -> String {
    let today_unix_days = super::date_utils::today_unix_days();
    let trends = compute_plot_trends(plot, today_unix_days);

    let mut out = String::new();
    out.push_str("\n");
    out.push_str(&format!("Milestone: {}\n", plot.milestone_name));
    out.push_str("\n");
    if ascii {
        out.push_str(&render_ascii_chart(&trends, fit, color));
    } else {
        out.push_str(&render_textplots_chart(&trends, fit, color));
    }
    out.push_str(&render_plot_legend(ascii, color));
    out.push_str("\n");
    out.push_str(&render_plot_trend_equations(&trends, color));
    if let Some(latest) = plot.points.last() {
        out.push_str("\n");
        out.push_str(&render_plot_stats(latest));
    }
    out.push_str("\n");
    out.push_str(&render_eta_text(
        trends.eta(today_unix_days),
        today_unix_days,
    ));
    out
}

/// Builds one series as `(x, y)` `f32` pairs ready for `textplots`, pairing
/// each sampled point with its already-computed x value. Shared by the
/// total/done data series (which otherwise differ only in which weight
/// field they read).
fn series_f32(
    points: &[TodoDonePlotPoint],
    x_values: &[f64],
    value_of: impl Fn(&TodoDonePlotPoint) -> f64,
) -> Vec<(f32, f32)> {
    points
        .iter()
        .zip(x_values.iter())
        .map(|(p, x)| (*x as f32, value_of(p) as f32))
        .collect()
}

fn render_textplots_chart(trends: &PlotTrends, fit: bool, color: bool) -> String {
    let points = &trends.sampled;
    let geometry = &trends.geometry;
    let total_series = series_f32(points, &geometry.x_values, |p| p.total_weight_wt);
    let done_series = series_f32(points, &geometry.x_values, |p| p.done_weight_wt);
    let total_trend_series = trends
        .total_trend
        .map(|t| trend_line_endpoints_f32(t, geometry.trend_end_x).to_vec())
        .unwrap_or_default();
    let done_trend_series = trends
        .done_trend
        .map(|t| trend_line_endpoints_f32(t, geometry.trend_end_x).to_vec())
        .unwrap_or_default();
    let xmax = geometry.chart_x_max as f32;
    let (ymin, ymax) = trends.y_range(fit);
    let (ymin, ymax) = (ymin as f32, ymax as f32);
    let today_series = vec![
        (geometry.today_x as f32, ymin),
        (geometry.today_x as f32, ymax),
    ];
    log::debug!(
        "render_textplots_chart: {CHART_CHAR_WIDTH}x{CHART_CHAR_HEIGHT} char canvas ({CHART_PIXEL_WIDTH}x{CHART_PIXEL_HEIGHT} px), total series ({} points), done series ({} points), today_x={:.3}, x range=[0, {xmax:.3}], y range=[{ymin:.3}, {ymax:.3}]",
        total_series.len(),
        done_series.len(),
        geometry.today_x
    );

    let total_line_shape = Shape::Lines(&total_series);
    let done_line_shape = Shape::Lines(&done_series);
    let total_point_shape = Shape::Points(&total_series);
    let done_point_shape = Shape::Points(&done_series);
    let total_trend_shape = Shape::Lines(&total_trend_series);
    let done_trend_shape = Shape::Lines(&done_trend_series);
    let today_shape = Shape::Lines(&today_series);
    let mut chart =
        Chart::new_with_y_range(CHART_PIXEL_WIDTH, CHART_PIXEL_HEIGHT, 0.0, xmax, ymin, ymax);
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
    if color {
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
    } else {
        // `textplots` only tells lines apart by color; without it every
        // series draws as the same plain Braille dot, so lines that
        // overlap become indistinguishable. `--ascii` is the recommended
        // way to keep the four lines distinguishable without color.
        if !total_trend_series.is_empty() {
            chart_ref = chart_ref.lineplot(&total_trend_shape);
        }
        if !done_trend_series.is_empty() {
            chart_ref = chart_ref.lineplot(&done_trend_shape);
        }
        chart_ref = chart_ref.lineplot(&today_shape);
        chart_ref = chart_ref
            .lineplot(&total_line_shape)
            .lineplot(&done_line_shape)
            .lineplot(&total_point_shape)
            .lineplot(&done_point_shape);
    }
    chart_ref.axis();
    chart_ref.figures();
    format!("{chart_ref}\n")
}

/// Canvas size shared by both chart backends, expressed in terminal
/// character cells (columns x rows). [`render_ascii_chart`] draws one
/// glyph per cell directly at this size. [`render_textplots_chart`]'s
/// Braille-based canvas packs a 2x4 sub-pixel grid into every terminal
/// cell, so it multiplies this size up into pixels (see
/// [`CHART_PIXEL_WIDTH`]/[`CHART_PIXEL_HEIGHT`]) to occupy the same
/// on-screen footprint at higher resolution.
const CHART_CHAR_WIDTH: usize = 60;
const CHART_CHAR_HEIGHT: usize = 20;

/// Braille sub-pixel packing factor used by the `textplots` backend: each
/// terminal cell holds a 2 (columns) x 4 (rows) grid of sub-pixels.
const BRAILLE_SUBPIXELS_X: usize = 2;
const BRAILLE_SUBPIXELS_Y: usize = 4;

const CHART_PIXEL_WIDTH: u32 = (CHART_CHAR_WIDTH * BRAILLE_SUBPIXELS_X) as u32;
const CHART_PIXEL_HEIGHT: u32 = (CHART_CHAR_HEIGHT * BRAILLE_SUBPIXELS_Y) as u32;

/// Fixed pixel-grid size for [`render_ascii_chart`]. A plain grid of one
/// character per cell can't reach the Braille backend's resolution, but
/// works on any 7-bit-ASCII terminal with no Unicode/Braille/ANSI-color
/// support. Kept at the same [`CHART_CHAR_WIDTH`]/[`CHART_CHAR_HEIGHT`]
/// footprint as the default chart so both backends render the same size.
const ASCII_CHART_WIDTH: usize = CHART_CHAR_WIDTH;
const ASCII_CHART_HEIGHT: usize = CHART_CHAR_HEIGHT;

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

    /// Draws `trend`'s straight line segment (over `x in [0, trend_end_x]`)
    /// onto the canvas, if `trend` was successfully fit. Shared by the
    /// total/done trend lines, which otherwise differ only in glyph/color.
    fn draw_trend_line(
        &mut self,
        trend: Option<LinearTrend>,
        trend_end_x: f64,
        ch: char,
        color: (u8, u8, u8),
    ) {
        let Some(t) = trend else { return };
        let e = trend_line_endpoints(t, trend_end_x);
        self.draw_line(e.x0, e.y0, e.x1, e.y1, ch, color);
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
/// Color is still applied (matching the plot legend's palette) for
/// terminals that do support it; symbols alone carry the same information
/// otherwise. Resolution is intentionally much lower than the default
/// Braille-based chart: one glyph per terminal cell instead of a packed
/// sub-pixel grid.
fn render_ascii_chart(trends: &PlotTrends, fit: bool, color: bool) -> String {
    let points = &trends.sampled;
    let geometry = &trends.geometry;
    let width = ASCII_CHART_WIDTH;
    let height = ASCII_CHART_HEIGHT;
    let xmax = geometry.chart_x_max.max(1.0);
    let (ymin, ymax) = trends.y_range(fit);
    let yspan = (ymax - ymin).max(1e-9);
    log::debug!(
        "render_ascii_chart: {}x{} grid, {} raw points, today_x={:.3}, x range=[0, {xmax:.3}], y range=[{ymin:.3}, {ymax:.3}]",
        width,
        height,
        points.len(),
        geometry.today_x
    );

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
    canvas.draw_trend_line(trends.total_trend, geometry.trend_end_x, 'O', (255, 255, 0));
    canvas.draw_trend_line(trends.done_trend, geometry.trend_end_x, '0', (0, 255, 255));

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
                Some(glyph) => match glyph.color.filter(|_| color) {
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
    // Match render_textplots_chart's trailing blank line before the legend.
    out.push('\n');
    out
}
