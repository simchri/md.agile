use super::*;

#[test]
fn render_trend_equations_shows_intercept_and_slope_relative_to_the_anchor_date() {
    let total_trend = LinearTrend {
        slope_wtpd: 1.95,
        intercept_wt: 29.85,
        anchor_unix_days: Some(20_000),
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.84,
        intercept_wt: 11.24,
        anchor_unix_days: Some(20_000),
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
fn render_trend_equations_shows_unknown_for_unfittable_trends() {
    let text = render_trend_equations(None, None, true);
    let occurrences = text.matches("unknown").count();
    assert_eq!(
        occurrences, 2,
        "both trend lines should render as 'unknown': {text:?}"
    );
}
