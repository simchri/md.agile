//! Combines a milestone's pure trend-line math (see `trend.rs`) with the
//! rendering-only detail (downsampling for display, chart axis geometry —
//! see `plot_data.rs`) that only becomes relevant once a plot is actually
//! drawn. Every chart backend renders from a [`ChartTrends`]; nothing
//! upstream of rendering (ETA, velocity/creep) depends on this module.

use super::eta_math::{EtaEstimate, compute_eta};
use super::plot_data::{
    PlotGeometry, TodoDonePlot, TodoDonePlotPoint, compute_plot_geometry, compute_plot_y_range,
    downsample_plot_points,
};
use super::trend::{LinearTrend, compute_milestone_trends};

/// Maximum number of points a chart draws directly; the milestone's full
/// history (however long) is downsampled to this many points purely for
/// display — trend fitting itself (see [`compute_milestone_trends`]) always
/// uses the full, undownsampled history.
const MAX_CHART_POINTS: usize = 96;

/// The trend lines and chart-rendering geometry/sampling used to draw a
/// milestone's chart (terminal or HTML/SVG). The trend lines themselves are
/// fit on the milestone's full point history (see `trend::MilestoneTrends`);
/// only the sampled points and axis geometry here are rendering-specific.
pub(super) struct ChartTrends {
    pub(super) sampled: Vec<TodoDonePlotPoint>,
    pub(super) geometry: PlotGeometry,
    pub(super) total_trend: Option<LinearTrend>,
    pub(super) done_trend: Option<LinearTrend>,
}

pub(super) fn compute_chart_trends(
    plot: &TodoDonePlot,
    today_unix_days: Option<i64>,
) -> ChartTrends {
    let milestone_trends = compute_milestone_trends(plot);
    let sampled = downsample_plot_points(&plot.points, MAX_CHART_POINTS);
    log::debug!(
        "compute_chart_trends: downsampled {} points to {} for display",
        plot.points.len(),
        sampled.len()
    );
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
        total_trend: milestone_trends.total_trend,
        done_trend: milestone_trends.done_trend,
    }
}

impl ChartTrends {
    /// Computes this chart's ETA (see [`compute_eta`]) from its fitted
    /// trend lines and calendar anchor date.
    pub(super) fn eta(&self, today_unix_days: Option<i64>) -> Option<EtaEstimate> {
        compute_eta(
            self.total_trend,
            self.done_trend,
            self.geometry.anchor_unix_days,
            today_unix_days,
        )
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
