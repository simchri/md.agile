use super::*;

#[test]
fn format_days_as_span_uses_days_below_one_week() {
    assert_eq!(format_days_as_span(1), "1 day");
    assert_eq!(format_days_as_span(6), "6 days");
}

#[test]
fn format_days_as_span_uses_weeks_below_eight_weeks() {
    assert_eq!(format_days_as_span(7), "1 week");
    assert_eq!(format_days_as_span(21), "3 weeks");
    assert_eq!(format_days_as_span(55), "8 weeks");
}

#[test]
fn format_days_as_span_uses_months_below_three_years() {
    assert_eq!(format_days_as_span(56), "2 months");
    assert_eq!(format_days_as_span(120), "4 months");
}

#[test]
fn format_days_as_span_uses_years_from_three_years() {
    assert_eq!(format_days_as_span(365 * 3), "3 years");
    assert_eq!(format_days_as_span(365 * 6), "6 years");
}

#[test]
fn compute_eta_intersects_trend_lines_in_the_future() {
    // total: flat at 10 (never grows); done: starts at 0, +1/day.
    let total_trend = LinearTrend {
        slope_wtpd: 0.0,
        intercept_wt: 10.0,
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.0,
        intercept_wt: 0.0,
    };
    // Anchor is day 0; "today" is day 0 too. Lines cross at x = 10.
    let eta = compute_eta(Some(total_trend), Some(done_trend), Some(0), Some(0))
        .expect("expected an ETA");
    assert_eq!(eta.unix_days, 10);
}

#[test]
fn compute_eta_is_none_when_trend_lines_are_parallel() {
    let total_trend = LinearTrend {
        slope_wtpd: 1.0,
        intercept_wt: 10.0,
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.0,
        intercept_wt: 0.0,
    };
    assert!(compute_eta(Some(total_trend), Some(done_trend), Some(0), Some(0)).is_none());
}

#[test]
fn compute_eta_is_none_when_intersection_is_in_the_past() {
    let total_trend = LinearTrend {
        slope_wtpd: 0.0,
        intercept_wt: 10.0,
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.0,
        intercept_wt: 0.0,
    };
    // Intersection is at x = 10 (relative to anchor), but "today" is day 20,
    // i.e. the crossing already happened in the past.
    assert!(compute_eta(Some(total_trend), Some(done_trend), Some(0), Some(20)).is_none());
}

#[test]
fn compute_eta_is_none_without_both_trends_or_anchor() {
    let total_trend = LinearTrend {
        slope_wtpd: 0.0,
        intercept_wt: 10.0,
    };
    assert!(compute_eta(Some(total_trend), None, Some(0), Some(0)).is_none());
    let done_trend = LinearTrend {
        slope_wtpd: 1.0,
        intercept_wt: 0.0,
    };
    assert!(compute_eta(Some(total_trend), Some(done_trend), None, Some(0)).is_none());
}

#[test]
fn render_eta_text_shows_span_and_date_when_available() {
    // unix_days = 10; "today" = 3, so 7 days remain -> "1 week".
    let eta = EtaEstimate { unix_days: 10 };
    let out = render_eta_text(Some(eta), Some(3));
    assert_eq!(out, "ETA:      1 week\nETA date: 1970-01-11\n");
}

#[test]
fn render_eta_text_shows_unknown_when_no_eta() {
    let out = render_eta_text(None, Some(0));
    assert_eq!(out, "ETA:      unknown\n");
}

#[test]
fn render_eta_text_shows_unknown_when_today_is_unknown() {
    let eta = EtaEstimate { unix_days: 10 };
    let out = render_eta_text(Some(eta), None);
    assert_eq!(out, "ETA:      unknown\n");
}
