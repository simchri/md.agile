//! Combines a milestone's pure trend-line math (see `trend.rs`) with
//! rendering-only chart axis geometry (see `plot_data.rs`) that only becomes
//! relevant once a plot is actually drawn. Every chart backend renders from
//! a [`ChartTrends`]; nothing upstream of rendering (ETA, velocity/creep)
//! depends on this module. Downsampling the point series for display (see
//! [`super::plot_data::downsample_plot_points`]) is a separate, purely
//! rendering-only concern and is the caller's responsibility: it happens
//! before [`compute_chart_trends`] is called, not inside it.

use super::eta_math::{EtaEstimate, compute_eta};
use super::plot_data::{
    PlotGeometry, TodoDonePlot, TodoDonePlotPoint, compute_plot_geometry, compute_plot_y_range,
};
use super::trend::{LinearTrend, compute_milestone_trends};

/// The trend lines and chart-rendering geometry used to draw a milestone's
/// chart (terminal or HTML/SVG). The trend lines themselves are fit on the
/// milestone's full point history (see `trend::MilestoneTrends`); only the
/// axis geometry here (built from `sampled`, whatever point series the
/// caller has chosen to draw) is rendering-specific.
pub(super) struct ChartTrends {
    pub(super) sampled: Vec<TodoDonePlotPoint>,
    pub(super) geometry: PlotGeometry,
    pub(super) total_trend: Option<LinearTrend>,
    pub(super) done_trend: Option<LinearTrend>,
}

/// Computes `plot`'s trend lines (fit on its full, undownsampled point
/// history) plus the chart geometry for `sampled` — the point series the
/// caller has already chosen to draw (e.g. downsampled for display via
/// [`super::plot_data::downsample_plot_points`], or the full history
/// unchanged). This function performs no sampling itself.
pub(super) fn compute_chart_trends(
    plot: &TodoDonePlot,
    sampled: Vec<TodoDonePlotPoint>,
    today_unix_days: Option<i64>,
) -> ChartTrends {
    let (total_trend, done_trend) = compute_milestone_trends(plot);
    let geometry = compute_plot_geometry(&sampled, today_unix_days);
    log::debug!(
        "compute_chart_trends: geometry = trend_end_x={:.3} today_x={:.3} chart_x_max={:.3} anchor_unix_days={:?}",
        geometry.trend_end_x,
        geometry.today_x,
        geometry.chart_x_max,
        geometry.anchor_unix_days
    );
    ChartTrends {
        sampled,
        geometry,
        total_trend,
        done_trend,
    }
}

impl ChartTrends {
    /// Computes this chart's ETA (see [`compute_eta`]) from its fitted
    /// trend lines.
    pub(super) fn eta(&self, today_unix_days: Option<i64>) -> Option<EtaEstimate> {
        compute_eta(self.total_trend, self.done_trend, today_unix_days)
    }

    /// Computes the y-axis range (see [`compute_plot_y_range`]) shared by
    /// every chart renderer, from this chart's sampled points, geometry,
    /// and fitted trend lines.
    pub(super) fn y_range(&self, fit: bool) -> (f64, f64) {
        compute_plot_y_range(
            &self.sampled,
            &self.geometry,
            self.total_trend,
            self.done_trend,
            fit,
        )
    }
}
