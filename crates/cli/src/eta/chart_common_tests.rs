use super::*;

#[test]
fn render_trend_equations_shows_intercept_and_slope_relative_to_the_anchor_date() {
    let total_trend = LinearTrend {
        slope_wtpd: 1.95,
        anchor_y_wt: 29.85,
        anchor_x_d: 20_000.0,
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.84,
        anchor_y_wt: 11.24,
        anchor_x_d: 20_000.0,
    };
    let text = render_trend_equations(Some(total_trend), Some(done_trend), true);
    assert!(
        text.contains("x = weeks since"),
        "text should explain what x means: {text:?}"
    );
    assert!(
        text.contains("29.85 + 13.65/week * x"),
        "text should show the total trend's equation, slope_wtpd in weight/week: {text:?}"
    );
    assert!(
        text.contains("11.24 + 12.88/week * x"),
        "text should show the done trend's equation, slope_wtpd in weight/week: {text:?}"
    );
}

#[test]
fn render_trend_equations_falls_back_to_point_index_without_an_anchor_date() {
    let text = render_trend_equations(None, None, true);
    assert!(
        text.contains("x = point index"),
        "text should fall back to point-index wording without real dates: {text:?}"
    );
}

#[test]
fn render_plot_stats_shows_percentage_of_tasks_and_weight_done() {
    let point = TodoDonePlotPoint {
        date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        total_weight_wt: 8.0,
        done_weight_wt: 2.0,
        total_count_t: 5,
        done_count_t: 1,
        total_top_level_t: 4,
        done_top_level_t: 1,
    };
    let text = render_plot_stats(&point);
    assert!(
        text.contains("25%"),
        "should show 1/4 top-level tasks done = 25%: {text:?}"
    );
    assert!(
        text.contains("25.00%") || text.contains("25%"),
        "should show 2/8 weight done = 25%: {text:?}"
    );
}

#[test]
fn render_plot_stats_percentage_is_zero_when_nothing_is_in_scope() {
    let point = TodoDonePlotPoint {
        date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        total_weight_wt: 0.0,
        done_weight_wt: 0.0,
        total_count_t: 0,
        done_count_t: 0,
        total_top_level_t: 0,
        done_top_level_t: 0,
    };
    let text = render_plot_stats(&point);
    assert!(
        text.contains("0%"),
        "should not divide by zero and instead show 0%: {text:?}"
    );
}
