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
    /// The line's value at `x = 0` (in the line's own coordinate space) —
    /// the weight (`_wt`) intercept.
    pub(super) anchor_y_wt: f64,
    /// The real calendar date that `x = 0` maps to, expressed as "unix
    /// days" (the `_d` suffix): the number of whole days since the Unix
    /// epoch (1970-01-01), i.e. `unix_seconds / 86_400`. This is the
    /// day-granularity analogue of a Unix timestamp — a single number
    /// that unambiguously identifies a calendar date, convenient for date
    /// arithmetic (differences, offsets) without touching time zones or
    /// sub-day precision. Not optional: a [`LinearTrend`] only exists once
    /// it's been fit through at least two dated points, so an anchor date
    /// is always available wherever a trend line itself is.
    pub(super) anchor_x_d: f64,
}

/// The trend-fitting algorithm to use when turning a series of dated
/// points into a [`LinearTrend`]. All algorithms fit a straight line (an
/// anchor point + slope, as [`LinearTrend`] itself assumes) — they differ
/// only in *how* that line is derived from the underlying points, e.g.
/// weighting recent points more heavily. Adding a new algorithm means
/// adding a variant here and a match arm in [`TrendFitAlgorithm::fit`]; no
/// other module needs to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrendFitAlgorithm {
    /// Ordinary least squares over the full point history, weighting every
    /// point equally.
    OrdinaryLeastSquares,
    /// Recency-weighted least squares (see [`recency_weighted_linear_trend`]
    /// for the exact weighting): the default, since it favors the
    /// project's *current* pace over its whole history.
    #[default]
    RecencyWeighted,
    /// Placeholder for algorithms not yet implemented: always returns
    /// `None`, so callers see "no trend" rather than a misleading fitted
    /// line. Not wired up to any production call site yet — exercised
    /// only by tests — so it's allowed to look unused outside `cfg(test)`
    /// builds.
    #[allow(dead_code)]
    Dummy,
}

impl TrendFitAlgorithm {
    /// Fits a [`LinearTrend`] through `points` (each an `(x, y)` pair, `x`
    /// in days since `anchor_x_d`), using this algorithm.
    fn fit(self, points: &[(f64, f64)], anchor_x_d: f64) -> Option<LinearTrend> {
        match self {
            TrendFitAlgorithm::OrdinaryLeastSquares => ols_linear_trend(points, anchor_x_d),
            TrendFitAlgorithm::RecencyWeighted => recency_weighted_linear_trend(points, anchor_x_d),
            TrendFitAlgorithm::Dummy => None,
        }
    }
}

/// Fits both the total and done trend lines (weight vs. time) for a
/// milestone from its full point history, using the given
/// [`TrendFitAlgorithm`] — pure math, no plotting/rendering detail
/// whatsoever (no sampling, no chart geometry): those only matter once a
/// plot is actually drawn.
pub(super) fn compute_milestone_trends_with(
    plot: &TodoDonePlot,
    algorithm: TrendFitAlgorithm,
) -> (Option<LinearTrend>, Option<LinearTrend>) {
    log::debug!(
        "compute_milestone_trends: plot.points has {} points (milestone {:?}) using {algorithm:?}",
        plot.points.len(),
        plot.milestone_name
    );
    let (x_values, anchor_x_d) = date_x_values(&plot.points);
    // Only used to construct a `LinearTrend` when there are >= 2 points, in
    // which case `date_x_values` always returns `Some` — the fallback here
    // is never observed.
    let anchor_x_d = anchor_x_d.unwrap_or(0) as f64;
    let total_trend = fit_series_trend(&x_values, &plot.points, anchor_x_d, algorithm, |p| {
        p.total_weight_wt
    });
    let done_trend = fit_series_trend(&x_values, &plot.points, anchor_x_d, algorithm, |p| {
        p.done_weight_wt
    });
    log::debug!("compute_milestone_trends: total_trend = {total_trend:?}");
    log::debug!("compute_milestone_trends: done_trend = {done_trend:?}");
    (total_trend, done_trend)
}

/// Maps each point's calendar date to an x value in days since the first
/// point's date, alongside that anchor date itself (as unix days — see
/// [`LinearTrend::anchor_x_d`] for what "unix days" means). `None` only
/// when there are no points at all. Shared by trend fitting here and,
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
/// pairing each point with its already-computed x value, using `algorithm`.
/// Shared by both `total_trend` and `done_trend` in
/// [`compute_milestone_trends_with`] so the two series are always fit the
/// exact same way.
fn fit_series_trend(
    x_values: &[f64],
    points: &[TodoDonePlotPoint],
    anchor_x_d: f64,
    algorithm: TrendFitAlgorithm,
    value_of: impl Fn(&TodoDonePlotPoint) -> f64,
) -> Option<LinearTrend> {
    algorithm.fit(
        &x_values
            .iter()
            .zip(points.iter())
            .map(|(x, p)| (*x, value_of(p)))
            .collect::<Vec<_>>(),
        anchor_x_d,
    )
}

/// The ordinary-least-squares fit: the classic "line of best fit" through
/// all points, weighting every point equally regardless of recency.
fn ols_linear_trend(points: &[(f64, f64)], anchor_x_d: f64) -> Option<LinearTrend> {
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
    let anchor_y_wt = mean_y - slope_wtpd * mean_x;
    Some(LinearTrend {
        slope_wtpd,
        anchor_y_wt,
        anchor_x_d,
    })
}

/// Keeps only the last point for each distinct `x` (day) in `points`,
/// preserving the order of each day's first appearance. `points` are
/// assumed to already be in chronological order, so when a day repeats
/// (multiple commits on the same day), the point kept is whichever one
/// comes last in `points` — i.e. that day's final recorded state.
fn dedupe_last_point_per_day(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut deduped: Vec<(f64, f64)> = Vec::new();
    for &(x, y) in points {
        match deduped
            .iter_mut()
            .find(|(ex, _)| (*ex - x).abs() <= f64::EPSILON)
        {
            Some(existing) => existing.1 = y,
            None => deduped.push((x, y)),
        }
    }
    deduped
}

/// Recency-weighted least squares: first [`dedupe_last_point_per_day`]s
/// `points` down to one point per calendar day (so a day with many commits
/// doesn't get more say in the fit than a quiet day), then fits a weighted
/// line where each deduped point's weight is its rank among them —
/// oldest = 1, up to newest = N — so more recent points pull the line
/// harder than older ones, favoring the project's current pace over its
/// whole history.
fn recency_weighted_linear_trend(points: &[(f64, f64)], anchor_x_d: f64) -> Option<LinearTrend> {
    let points = dedupe_last_point_per_day(points);
    if points.len() < 2 {
        return None;
    }
    let weights: Vec<f64> = (1..=points.len()).map(|rank| rank as f64).collect();
    let sum_w: f64 = weights.iter().sum();
    let mean_x = points
        .iter()
        .zip(&weights)
        .map(|((x, _), w)| w * x)
        .sum::<f64>()
        / sum_w;
    let mean_y = points
        .iter()
        .zip(&weights)
        .map(|((_, y), w)| w * y)
        .sum::<f64>()
        / sum_w;
    let mut cov = 0.0;
    let mut var = 0.0;
    for ((x, y), w) in points.iter().zip(&weights) {
        cov += w * (x - mean_x) * (y - mean_y);
        var += w * (x - mean_x) * (x - mean_x);
    }
    if var <= f64::EPSILON {
        return None;
    }
    let slope_wtpd = cov / var;
    let anchor_y_wt = mean_y - slope_wtpd * mean_x;
    Some(LinearTrend {
        slope_wtpd,
        anchor_y_wt,
        anchor_x_d,
    })
}

#[cfg(test)]
#[path = "trend_tests.rs"]
mod tests;
