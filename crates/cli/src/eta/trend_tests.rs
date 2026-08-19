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
fn ordinary_least_squares_is_the_default_algorithm() {
    let plot = plot_with_points(vec![(date(1), 10.0, 0.0), (date(2), 10.0, 1.0)]);
    let (default_total, default_done) = compute_milestone_trends(&plot);
    let (explicit_total, explicit_done) =
        compute_milestone_trends_with(&plot, TrendFitAlgorithm::OrdinaryLeastSquares);
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
