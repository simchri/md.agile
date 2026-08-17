//! ETA math: computing the calendar date (as unix days) where a
//! milestone's total and done trend lines intersect. Pure date/time math —
//! no string formatting (see `eta_text.rs` for that) and no trend-fitting
//! (see `trend.rs`).

use super::TodoDonePlot;
use super::trend::{LinearTrend, compute_milestone_trends};

/// The estimated time of arrival at a milestone: the calendar date (as unix
/// days) where the total and done trend lines intersect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EtaEstimate {
    pub(super) unix_days: i64,
}

/// Computes a milestone's ETA (see [`compute_eta`]) directly from its plot
/// data, deriving the trend lines the same way every other consumer does.
pub(super) fn eta_for_plot(
    plot: &TodoDonePlot,
    today_unix_days: Option<i64>,
) -> Option<EtaEstimate> {
    let (total_trend, done_trend) = compute_milestone_trends(plot);
    compute_eta(total_trend, done_trend, today_unix_days)
}

/// Computes the ETA to a milestone as the intersection of the total and done
/// trend lines, expressed relative to their shared `anchor_x_d` (the
/// calendar date that trend-line x = 0 maps to). Returns `None` when either
/// trend line is missing, the lines are parallel (no single intersection),
/// the anchor date couldn't be determined (e.g. no real dates available),
/// or the intersection falls on or before today (already reached, or
/// unknowable).
///
/// This function is purely date/time math — it performs no string
/// formatting; see `eta_text::render_eta_text` for that.
pub(super) fn compute_eta(
    total_trend: Option<LinearTrend>,
    done_trend: Option<LinearTrend>,
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
    let Some(anchor) = total.anchor_x_d else {
        log::debug!("compute_eta: no anchor_x_d available -> None");
        return None;
    };
    let Some(today) = today_unix_days else {
        log::debug!("compute_eta: no today_unix_days available -> None");
        return None;
    };

    log::debug!(
        "compute_eta: total_trend={total:?} done_trend={done:?} anchor_x_d={anchor} today_unix_days={today}"
    );

    let slope_diff = total.slope_wtpd - done.slope_wtpd;
    if slope_diff.abs() <= f64::EPSILON {
        log::debug!("compute_eta: slopes are equal (parallel trend lines) -> None");
        return None;
    }
    let x_intersect = (done.anchor_y_wt - total.anchor_y_wt) / slope_diff;
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

#[cfg(test)]
#[path = "eta_math_tests.rs"]
mod tests;
