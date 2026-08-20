use super::*;
use chrono::NaiveDate;

fn plot_with_points(points: Vec<(NaiveDate, f64, f64)>) -> TodoDonePlot {
    TodoDonePlot {
        milestone_name: "test milestone".to_string(),
        points: points
            .into_iter()
            .map(
                |(date, total_weight_wt, done_weight_wt)| TodoDonePlotPoint {
                    date,
                    total_weight_wt,
                    done_weight_wt,
                    total_count_t: 0,
                    done_count_t: 0,
                },
            )
            .collect(),
    }
}

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, day).unwrap()
}

#[test]
fn recency_weighted_is_the_default_algorithm() {
    let plot = plot_with_points(vec![(date(1), 10.0, 0.0), (date(2), 10.0, 1.0)]);
    let (default_total, default_done) =
        compute_milestone_trends_with(&plot, TrendFitAlgorithm::default());
    let (explicit_total, explicit_done) =
        compute_milestone_trends_with(&plot, TrendFitAlgorithm::RecencyWeighted);
    assert_eq!(default_total, explicit_total);
    assert_eq!(default_done, explicit_done);
    assert!(explicit_total.is_some());
    assert!(explicit_done.is_some());
}

#[test]
fn ordinary_least_squares_needs_at_least_two_points() {
    let plot = plot_with_points(vec![(date(1), 10.0, 0.0)]);
    let (total, done) =
        compute_milestone_trends_with(&plot, TrendFitAlgorithm::OrdinaryLeastSquares);
    assert!(total.is_none());
    assert!(done.is_none());
}

#[test]
fn recency_weighted_needs_at_least_two_distinct_days() {
    // Two points, but on the same day: they dedupe down to a single point,
    // which isn't enough to fit a line.
    let plot = plot_with_points(vec![(date(1), 10.0, 0.0), (date(1), 10.0, 5.0)]);
    let (total, done) = compute_milestone_trends_with(&plot, TrendFitAlgorithm::RecencyWeighted);
    assert!(total.is_none());
    assert!(done.is_none());
}

#[test]
fn recency_weighted_dedupes_same_day_points_keeping_the_last() {
    // Two points on day 1 (an earlier and a later commit that day), then
    // one point on day 2. Deduped down to exactly two points — the later
    // day-1 value and the day-2 value — a line through exactly two points
    // is unaffected by weighting, so this isolates the dedup behavior.
    let with_duplicate = plot_with_points(vec![
        (date(1), 10.0, 0.0),
        (date(1), 10.0, 4.0),
        (date(2), 10.0, 6.0),
    ]);
    let without_duplicate = plot_with_points(vec![(date(1), 10.0, 4.0), (date(2), 10.0, 6.0)]);

    let (_, done_with_duplicate) =
        compute_milestone_trends_with(&with_duplicate, TrendFitAlgorithm::RecencyWeighted);
    let (_, done_without_duplicate) =
        compute_milestone_trends_with(&without_duplicate, TrendFitAlgorithm::RecencyWeighted);

    assert_eq!(done_with_duplicate, done_without_duplicate);
    assert!((done_with_duplicate.unwrap().slope_wtpd - 2.0).abs() < 1e-9);
}

#[test]
fn recency_weighted_gives_more_weight_to_recent_points_than_ordinary_least_squares() {
    // A late jump (day 2) should pull a recency-weighted fit's slope up
    // more than an unweighted (OLS) fit's, since OLS treats every day
    // equally regardless of how recent it is.
    let plot = plot_with_points(vec![
        (date(1), 10.0, 0.0),
        (date(2), 10.0, 0.0),
        (date(3), 10.0, 10.0),
    ]);
    let (_, ols_done) =
        compute_milestone_trends_with(&plot, TrendFitAlgorithm::OrdinaryLeastSquares);
    let (_, recency_done) =
        compute_milestone_trends_with(&plot, TrendFitAlgorithm::RecencyWeighted);

    let ols_slope = ols_done.unwrap().slope_wtpd;
    let recency_slope = recency_done.unwrap().slope_wtpd;
    assert!(
        recency_slope > ols_slope,
        "expected recency-weighted slope ({recency_slope}) to exceed OLS slope ({ols_slope})"
    );
}

#[test]
fn exponential_decay_needs_at_least_two_distinct_days() {
    // Same dedup rule as `RecencyWeighted`: two same-day points collapse
    // into one, which isn't enough to fit a line.
    let plot = plot_with_points(vec![(date(1), 10.0, 0.0), (date(1), 10.0, 5.0)]);
    let (total, done) = compute_milestone_trends_with(&plot, TrendFitAlgorithm::ExponentialDecay);
    assert!(total.is_none());
    assert!(done.is_none());
}

#[test]
fn exponential_decay_dedupes_same_day_points_keeping_the_last() {
    // Mirrors `recency_weighted_dedupes_same_day_points_keeping_the_last`:
    // deduping down to two points makes the fit a straight line through
    // exactly those two points, unaffected by weighting.
    let with_duplicate = plot_with_points(vec![
        (date(1), 10.0, 0.0),
        (date(1), 10.0, 4.0),
        (date(2), 10.0, 6.0),
    ]);
    let without_duplicate = plot_with_points(vec![(date(1), 10.0, 4.0), (date(2), 10.0, 6.0)]);

    let (_, done_with_duplicate) =
        compute_milestone_trends_with(&with_duplicate, TrendFitAlgorithm::ExponentialDecay);
    let (_, done_without_duplicate) =
        compute_milestone_trends_with(&without_duplicate, TrendFitAlgorithm::ExponentialDecay);

    assert_eq!(done_with_duplicate, done_without_duplicate);
    assert!((done_with_duplicate.unwrap().slope_wtpd - 2.0).abs() < 1e-9);
}

#[test]
fn exponential_decay_gives_more_weight_to_recent_points_than_recency_weighted() {
    // A late jump (day 5, the last of 5 days) should pull the exponential-
    // decay fit's slope up more aggressively than the linear-rank
    // `RecencyWeighted` fit's, since decay weighting concentrates far more
    // relative weight on the last day(s) than a linear 1..N ramp does.
    let plot = plot_with_points(vec![
        (date(1), 10.0, 0.0),
        (date(2), 10.0, 0.0),
        (date(3), 10.0, 0.0),
        (date(4), 10.0, 0.0),
        (date(5), 10.0, 20.0),
    ]);
    let (_, recency_done) =
        compute_milestone_trends_with(&plot, TrendFitAlgorithm::RecencyWeighted);
    let (_, decay_done) = compute_milestone_trends_with(&plot, TrendFitAlgorithm::ExponentialDecay);

    let recency_slope = recency_done.unwrap().slope_wtpd;
    let decay_slope = decay_done.unwrap().slope_wtpd;
    assert!(
        decay_slope > recency_slope,
        "expected exponential-decay slope ({decay_slope}) to exceed linear-rank \
         recency-weighted slope ({recency_slope})"
    );
}

#[test]
fn exponential_decay_half_life_scales_with_the_observed_span() {
    // Doubling every date (stretching the span from 4 days to 8 days,
    // scaling the half-life proportionally) while keeping the same
    // relative shape of values should reproduce the same slope in the
    // doubled x-coordinate space (half the original, since the same
    // total rise now happens over double the run).
    let narrow_plot = plot_with_points(vec![
        (date(1), 10.0, 0.0),
        (date(2), 10.0, 0.0),
        (date(3), 10.0, 0.0),
        (date(4), 10.0, 0.0),
        (date(5), 10.0, 20.0),
    ]);
    let wide_plot = plot_with_points(vec![
        (date(1), 10.0, 0.0),
        (date(3), 10.0, 0.0),
        (date(5), 10.0, 0.0),
        (date(7), 10.0, 0.0),
        (date(9), 10.0, 20.0),
    ]);

    let (_, narrow_done) =
        compute_milestone_trends_with(&narrow_plot, TrendFitAlgorithm::ExponentialDecay);
    let (_, wide_done) =
        compute_milestone_trends_with(&wide_plot, TrendFitAlgorithm::ExponentialDecay);

    let narrow_slope = narrow_done.unwrap().slope_wtpd;
    let wide_slope = wide_done.unwrap().slope_wtpd;
    assert!(
        (wide_slope - narrow_slope / 2.0).abs() < 1e-6,
        "expected wide-span slope ({wide_slope}) to be half the narrow-span \
         slope ({narrow_slope}) once the half-life scales with the span"
    );
}

#[test]
fn dummy_algorithm_always_returns_none() {
    let plot = plot_with_points(vec![
        (date(1), 10.0, 0.0),
        (date(2), 10.0, 1.0),
        (date(3), 10.0, 2.0),
    ]);
    let (total, done) = compute_milestone_trends_with(&plot, TrendFitAlgorithm::Dummy);
    assert!(total.is_none());
    assert!(done.is_none());
}
