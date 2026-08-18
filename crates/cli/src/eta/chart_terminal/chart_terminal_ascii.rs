//! The `--ascii` terminal chart backend: a plain 7-bit-ASCII character grid
//! (see [`super::chart_terminal_braille`] for the default, higher-resolution
//! Braille sub-pixel backend).

use super::super::chart_common::ansi_rgb_text;
use super::super::chart_trends::ChartTrends;
use super::super::plot_data::{TodoDonePlotPoint, x_axis_date_labels};
use super::super::trend::LinearTrend;
use super::super::trend_geometry::trend_line_endpoints;
use super::{CHART_CHAR_HEIGHT, CHART_CHAR_WIDTH};

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
    xmin: f64,
    xspan: f64,
    ymin: f64,
    yspan: f64,
}

impl AsciiCanvas {
    fn new(width: usize, height: usize, xmin: f64, xspan: f64, ymin: f64, yspan: f64) -> Self {
        Self {
            grid: vec![vec![None; width]; height],
            width,
            height,
            xmin,
            xspan,
            ymin,
            yspan,
        }
    }

    fn to_col(&self, x: f64) -> usize {
        let frac = ((x - self.xmin) / self.xspan).clamp(0.0, 1.0);
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

/// Renders the same total/done/trend/today lines
/// [`super::chart_terminal_braille::render_textplots_chart`] draws, but onto
/// a plain fixed-size character grid using only 7-bit ASCII symbols (`o`,
/// `@`, `O`, `0`, `Q`) — one distinct, large/round symbol per series, so the
/// chart stays readable even without ANSI color support. Color is still
/// applied (matching the plot legend's palette) for terminals that do
/// support it; symbols alone carry the same information otherwise.
/// Resolution is intentionally much lower than the default Braille-based
/// chart: one glyph per terminal cell instead of a packed sub-pixel grid.
pub(super) fn render_ascii_chart(trends: &ChartTrends, fit: bool, color: bool) -> String {
    let points = &trends.sampled;
    let geometry = &trends.geometry;
    let width = ASCII_CHART_WIDTH;
    let height = ASCII_CHART_HEIGHT;
    let xmin = geometry.chart_x_min;
    let xspan = (geometry.chart_x_max - xmin).max(1.0);
    let (ymin, ymax) = trends.y_range(fit);
    let yspan = (ymax - ymin).max(1e-9);
    log::debug!(
        "render_ascii_chart: {}x{} grid, {} raw points, today_x={:.3}, x range=[{xmin:.3}, {:.3}], y range=[{ymin:.3}, {ymax:.3}]",
        width,
        height,
        points.len(),
        geometry.today_x,
        xmin + xspan
    );

    let mut canvas = AsciiCanvas::new(width, height, xmin, xspan, ymin, yspan);

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
