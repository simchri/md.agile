use super::*;
use crate::eta::eta_text::{eta_span, render_eta_text};
use crate::eta::trend::LinearTrend;

#[test]
fn compute_eta_intersects_trend_lines_in_the_future() {
    // total: flat at 10 (never grows); done: starts at 0, +1/day.
    let total_trend = LinearTrend {
        slope_wtpd: 0.0,
        anchor_y_wt: 10.0,
        anchor_x_d: 0.0,
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.0,
        anchor_y_wt: 0.0,
        anchor_x_d: 0.0,
    };
    // Anchor is day 0; "today" is day 0 too. Lines cross at x = 10.
    let eta = compute_eta(Some(total_trend), Some(done_trend), Some(0.0)).expect("expected an ETA");
    assert_eq!(eta.unix_days, 10);
}

#[test]
fn compute_eta_is_none_when_trend_lines_are_parallel() {
    let total_trend = LinearTrend {
        slope_wtpd: 1.0,
        anchor_y_wt: 10.0,
        anchor_x_d: 0.0,
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.0,
        anchor_y_wt: 0.0,
        anchor_x_d: 0.0,
    };
    assert!(compute_eta(Some(total_trend), Some(done_trend), Some(0.0)).is_none());
}

#[test]
fn compute_eta_is_none_when_intersection_is_in_the_past() {
    let total_trend = LinearTrend {
        slope_wtpd: 0.0,
        anchor_y_wt: 10.0,
        anchor_x_d: 0.0,
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.0,
        anchor_y_wt: 0.0,
        anchor_x_d: 0.0,
    };
    // Intersection is at x = 10 (relative to anchor), but "today" is day 20,
    // i.e. the crossing already happened in the past.
    assert!(compute_eta(Some(total_trend), Some(done_trend), Some(20.0)).is_none());
}

#[test]
fn compute_eta_is_none_without_both_trends() {
    let total_trend = LinearTrend {
        slope_wtpd: 0.0,
        anchor_y_wt: 10.0,
        anchor_x_d: 0.0,
    };
    assert!(compute_eta(Some(total_trend), None, Some(0.0)).is_none());
    assert!(compute_eta(None, Some(total_trend), Some(0.0)).is_none());
}

#[test]
fn compute_eta_within_a_day_is_today_even_when_rounding_would_hide_it() {
    let total_trend = LinearTrend {
        slope_wtpd: 0.0,
        anchor_y_wt: 0.25,
        anchor_x_d: 10.0,
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.0,
        anchor_y_wt: 0.0,
        anchor_x_d: 10.0,
    };
    let eta = compute_eta(Some(total_trend), Some(done_trend), Some(10.1)).unwrap();
    assert_eq!(eta.unix_days, 10);
    assert_eq!(eta_span(Some(eta), Some(10)).as_deref(), Some("today"));
    assert!(compute_eta(Some(total_trend), Some(done_trend), Some(10.3)).is_none());
    assert!(compute_eta(Some(total_trend), Some(done_trend), Some(10.25)).is_none());
}

#[test]
fn compute_eta_within_a_day_across_midnight_keeps_actual_calendar_date() {
    let total_trend = LinearTrend {
        slope_wtpd: 0.0,
        anchor_y_wt: 0.1,
        anchor_x_d: 11.0,
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.0,
        anchor_y_wt: 0.0,
        anchor_x_d: 11.0,
    };
    let eta = compute_eta(Some(total_trend), Some(done_trend), Some(10.9)).unwrap();
    assert_eq!(eta.unix_days, 11);
    assert_eq!(eta_span(Some(eta), Some(10)).as_deref(), Some("today"));
    assert_eq!(
        render_eta_text(Some(eta), Some(10)),
        "ETA:      today\nETA date: 1970-01-12\n"
    );
}

#[test]
fn compute_eta_one_full_day_away_uses_a_day_span() {
    let total_trend = LinearTrend {
        slope_wtpd: 0.0,
        anchor_y_wt: 1.25,
        anchor_x_d: 10.0,
    };
    let done_trend = LinearTrend {
        slope_wtpd: 1.0,
        anchor_y_wt: 0.0,
        anchor_x_d: 10.0,
    };
    let eta = compute_eta(Some(total_trend), Some(done_trend), Some(10.25)).unwrap();
    assert!(!eta.within_day);
    assert_eq!(eta_span(Some(eta), Some(10)).as_deref(), Some("1 day"));
}
