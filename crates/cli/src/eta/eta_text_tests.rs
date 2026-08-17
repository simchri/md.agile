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
