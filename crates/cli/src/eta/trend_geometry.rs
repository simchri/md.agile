//! Converts a fitted [`LinearTrend`] (slope + intercept) into the two
//! endpoint coordinates needed to draw it as a straight line, over the `x in
//! [trend.anchor_x_d, trend_end_x]` window shared by all three chart
//! renderers (SVG/HTML, Braille/textplots, ASCII). Extracted here because
//! the exact same `(x0, y0) = (anchor_x_d, anchor_y_wt)` / `(x1, y1) =
//! (trend_end_x, slope * (trend_end_x - anchor_x_d) + anchor_y_wt)`
//! computation was previously duplicated independently in each renderer.
//! `trend_end_x` (like every other `x` throughout the graphing pipeline —
//! see `plot_data::PlotGeometry`) is a plain unix-days value, in the same
//! coordinate system as `anchor_x_d` itself; callers never need to
//! translate between a "relative to the plotted series" origin and this
//! one, because there isn't a separate one.

use super::trend::LinearTrend;

/// The two endpoints of a trend line's straight-line segment, in plot data
/// coordinates (not yet projected to pixel/SVG space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TrendLineEndpoints {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

/// Computes the two data-space endpoints of `trend`'s line over `x in
/// [trend.anchor_x_d, trend_end_x]`: `(anchor_x_d, anchor_y_wt)` and
/// `(trend_end_x, slope * (trend_end_x - anchor_x_d) + anchor_y_wt)`.
pub(super) fn trend_line_endpoints(trend: LinearTrend, trend_end_x: f64) -> TrendLineEndpoints {
    TrendLineEndpoints {
        x0: trend.anchor_x_d,
        y0: trend.anchor_y_wt,
        x1: trend_end_x,
        y1: trend.slope_wtpd * (trend_end_x - trend.anchor_x_d) + trend.anchor_y_wt,
    }
}

/// Same as [`trend_line_endpoints`], but returned as `(f32, f32)` pairs
/// ready for `textplots::Shape::Lines`, which only accepts `f32` points.
pub(super) fn trend_line_endpoints_f32(trend: LinearTrend, trend_end_x: f64) -> [(f32, f32); 2] {
    let e = trend_line_endpoints(trend, trend_end_x);
    [(e.x0 as f32, e.y0 as f32), (e.x1 as f32, e.y1 as f32)]
}

#[cfg(test)]
#[path = "trend_geometry_tests.rs"]
mod tests;
