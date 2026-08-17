//! Trend-line fitting and ETA math shared by every chart/report: fitting a
//! linear trend through a series of `(x, weight)` points, and computing/
//! formatting the ETA (the calendar date where the total and done trend
//! lines intersect).
//!
//! Everything here is pure math over a milestone's full point history: no
//! plotting/rendering detail (downsampling for display, chart axis
//! geometry) lives in this module. Those only become relevant once a plot
//! is actually rendered — see `plot_data::ChartTrends`, which wraps a
//! [`MilestoneTrends`] with that rendering-only detail for the chart
//! backends.

use super::date_utils::{date_from_unix_days, unix_days_from_date};
use super::{TodoDonePlot, TodoDonePlotPoint};

/// Number of days in a week, used to convert day-based rates (`_wtpd`) to
/// week-based rates (`_wtpw`) for display purposes.
pub(super) const DAYS_PER_WEEK: f64 = 7.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LinearTrend {
    pub(super) slope_wtpd: f64,
    pub(super) intercept_wt: f64,
}

/// The estimated time of arrival at a milestone: the calendar date (as unix
/// days) where the total and done trend lines intersect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EtaEstimate {
    pub(super) unix_days: i64,
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

impl MilestoneTrends {
    /// Computes this milestone's ETA (see [`compute_eta`]) from its
    /// already-fit trend lines. Shared by every renderer/report so they
    /// can't drift on how ETA is derived from a [`MilestoneTrends`].
    pub(super) fn eta(&self, today_unix_days: Option<i64>) -> Option<EtaEstimate> {
        compute_eta(
            self.total_trend,
            self.done_trend,
            self.anchor_unix_days,
            today_unix_days,
        )
    }
}

/// Computes a milestone's ETA (see [`compute_eta`]) directly from its plot
/// data, deriving the trend lines the same way every other consumer does.
pub(super) fn eta_for_plot(
    plot: &TodoDonePlot,
    today_unix_days: Option<i64>,
) -> Option<EtaEstimate> {
    compute_milestone_trends(plot).eta(today_unix_days)
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
pub(super) fn compute_eta(
    total_trend: Option<LinearTrend>,
    done_trend: Option<LinearTrend>,
    anchor_unix_days: Option<i64>,
    today_unix_days: Option<i64>,
) -> Option<EtaEstimate> {
    let Some(total) = total_trend else {
        log::debug!("compute_eta: no total trend available -> None");
        return None;
    };
    let Some(done) = done_trend else {
        log::debug!("compute_eta: no done trend available -> None");
        return None;
    };
    let Some(anchor) = anchor_unix_days else {
        log::debug!("compute_eta: no anchor_unix_days available -> None");
        return None;
    };
    let Some(today) = today_unix_days else {
        log::debug!("compute_eta: no today_unix_days available -> None");
        return None;
    };

    log::debug!(
        "compute_eta: total_trend={total:?} done_trend={done:?} anchor_unix_days={anchor} today_unix_days={today}"
    );

    let slope_diff = total.slope_wtpd - done.slope_wtpd;
    if slope_diff.abs() <= f64::EPSILON {
        log::debug!("compute_eta: slopes are equal (parallel trend lines) -> None");
        return None;
    }
    let x_intersect = (done.intercept_wt - total.intercept_wt) / slope_diff;
    let unix_days = anchor + x_intersect.round() as i64;
    log::debug!(
        "compute_eta: slope_diff={slope_diff:.6} x_intersect={x_intersect:.3} (days since anchor) -> unix_days={unix_days}"
    );

    if unix_days <= today {
        log::debug!(
            "compute_eta: intersection unix_days={unix_days} <= today={today} (already reached, or in the past) -> None"
        );
        return None;
    }

    log::debug!("compute_eta: -> Some(EtaEstimate {{ unix_days: {unix_days} }})");
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
pub(super) fn eta_span(eta: Option<EtaEstimate>, today_unix_days: Option<i64>) -> Option<String> {
    let (eta, today) = (eta?, today_unix_days?);
    Some(format_days_as_span(eta.unix_days - today))
}

/// Renders the "ETA: ..." / "ETA date: ..." text block shown after the plot.
/// All string formatting (date formatting and the day-count-to-span
/// conversion) lives here; [`compute_eta`] only ever deals in dates.
pub(super) fn render_eta_text(eta: Option<EtaEstimate>, today_unix_days: Option<i64>) -> String {
    let Some(span) = eta_span(eta, today_unix_days) else {
        return format!("{:<10}unknown\n", "ETA:");
    };
    let date = date_from_unix_days(eta.unwrap().unix_days)
        .map(|d| d.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!("{:<10}{span}\n{:<10}{date}\n", "ETA:", "ETA date:")
}

#[cfg(test)]
#[path = "trend_tests.rs"]
mod tests;
