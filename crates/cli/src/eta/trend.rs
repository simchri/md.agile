//! Trend-line fitting shared by every chart/report: fitting a linear trend
//! through a milestone's total/done weight-over-time series. Pure math over
//! a milestone's full point history: no plotting/rendering detail
//! (downsampling for display, chart axis geometry) lives in this module —
//! those only become relevant once a plot is actually rendered, see
//! `chart_trends.rs`. ETA math (turning two trend lines into a target date)
//! and its text formatting live separately, in `eta_math.rs`/`eta_text.rs`.

use super::date_utils::unix_days_from_date;
use super::{TodoDonePlot, TodoDonePlotPoint};

/// Number of days in a week, used to convert day-based rates (`_wtpd`) to
/// week-based rates (`_wtpw`) for display purposes.
pub(super) const DAYS_PER_WEEK: f64 = 7.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LinearTrend {
    pub(super) slope_wtpd: f64,
    pub(super) intercept_wt: f64,
}

/// The two fitted trend lines (total/done weight vs. time) for a milestone,
/// plus the calendar anchor date they're relative to — pure math, computed
/// from the milestone's full point history. Carries no plotting/rendering
/// detail whatsoever (no sampling, no chart geometry): those only matter
/// once a plot is actually drawn.
pub(super) struct MilestoneTrends {
    pub(super) total_trend: Option<LinearTrend>,
    pub(super) done_trend: Option<LinearTrend>,
    /// Real calendar date (as unix days) that x = 0 (day offset 0) maps to.
    /// `None` when the points don't carry parseable dates (e.g. in tests),
    /// in which case x values are plain indices and an ETA can't be
    /// resolved to a calendar date.
    pub(super) anchor_unix_days: Option<i64>,
}

pub(super) fn compute_milestone_trends(plot: &TodoDonePlot) -> MilestoneTrends {
    log::debug!(
        "compute_milestone_trends: plot.points has {} points (milestone {:?})",
        plot.points.len(),
        plot.milestone_name
    );
    let (x_values, anchor_unix_days) = date_x_values(&plot.points);
    let total_trend = fit_series_trend(&x_values, &plot.points, |p| p.total_weight_wt);
    let done_trend = fit_series_trend(&x_values, &plot.points, |p| p.done_weight_wt);
    log::debug!(
        "compute_milestone_trends: total_trend = {total_trend:?} (slope_wtpd, intercept_wt)"
    );
    log::debug!("compute_milestone_trends: done_trend = {done_trend:?} (slope_wtpd, intercept_wt)");
    MilestoneTrends {
        total_trend,
        done_trend,
        anchor_unix_days,
    }
}

/// Maps each point's calendar date to an x value in days since the first
/// point's date, alongside that anchor date itself (as unix days). `None`
/// only when there are no points at all. Shared by trend fitting here and,
/// independently, by `plot_data::compute_plot_geometry` (which applies the
/// same mapping to the downsampled/display point series for rendering).
pub(super) fn date_x_values(points: &[TodoDonePlotPoint]) -> (Vec<f64>, Option<i64>) {
    let Some(first_point) = points.first() else {
        return (Vec::new(), None);
    };
    let first_date_days = unix_days_from_date(first_point.date);
    let x_values = points
        .iter()
        .map(|point| (unix_days_from_date(point.date) - first_date_days) as f64)
        .collect();
    (x_values, Some(first_date_days))
}

/// Fits a [`LinearTrend`] through one plotted series (total or done weight),
/// pairing each point with its already-computed x value. Shared by both
/// `total_trend` and `done_trend` in [`compute_milestone_trends`] so the two
/// series are always fit the exact same way.
fn fit_series_trend(
    x_values: &[f64],
    points: &[TodoDonePlotPoint],
    value_of: impl Fn(&TodoDonePlotPoint) -> f64,
) -> Option<LinearTrend> {
    linear_trend(
        &x_values
            .iter()
            .zip(points.iter())
            .map(|(x, p)| (*x, value_of(p)))
            .collect::<Vec<_>>(),
    )
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
