//! Combines a milestone's pure trend-line math (see `trend.rs`) with the
//! rendering-only fields (sampled points, chart axis geometry — see
//! `plot_data.rs`) that only become relevant once a plot is actually drawn,
//! bundling them into one [`PlotData`] value plus convenience methods
//! ([`PlotData::eta`], [`PlotData::y_range`]) so every chart renderer
//! reads from the same place. Building a [`PlotData`] — downsampling for
//! display, fitting the trend lines, and computing the axis geometry — is
//! each renderer's own responsibility (see `chart_html.rs`,
//! `chart_terminal.rs`); this module holds no sampling or geometry logic of
//! its own.

use super::eta_math::{EtaEstimate, compute_eta};
use super::plot_data::{PlotGeometry, TodoDonePlotPoint, compute_plot_y_range};
use super::trend::LinearTrend;

/// The trend lines, sampled points, and chart-rendering geometry used to
/// draw a milestone's chart (terminal or HTML/SVG). The trend lines
/// themselves are fit on the milestone's full point history (see
/// `trend::MilestoneTrends`); `sampled` and `geometry` are rendering-only
/// and built by whichever chart renderer constructs this value.
pub(super) struct PlotData {
    pub(super) sampled: Vec<TodoDonePlotPoint>,
    pub(super) geometry: PlotGeometry,
    pub(super) total_trend: Option<LinearTrend>,
    pub(super) done_trend: Option<LinearTrend>,
}

impl PlotData {
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
