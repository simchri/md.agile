use super::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn parse_items(content: &str) -> Vec<FileItem> {
    parser::parse(content, PathBuf::from("tasks.agile.md"))
}

#[test]
fn completion_weight_delta_counts_top_level_todo_to_done() {
    let old_file_content = "\
- [ ] task
";
    let new_file_content = "\
- [x] task
";

    let old_items = parse_items(old_file_content);
    let new_items = parse_items(new_file_content);
    let (delta, events) = completion_weight_delta(&old_items, &new_items);

    assert_eq!(events, 1, "events: {events}");
    assert!((delta - 1.0).abs() < f64::EPSILON, "delta: {delta}");
}

#[test]
fn completion_weight_delta_counts_subtask_by_depth_weight() {
    let old_file_content = "\
- [ ] parent
  - [ ] child
";
    let new_file_content = "\
- [ ] parent
  - [x] child
";

    let old_items = parse_items(old_file_content);
    let new_items = parse_items(new_file_content);
    let (delta, events) = completion_weight_delta(&old_items, &new_items);

    assert_eq!(events, 1, "events: {events}");
    assert!((delta - 0.5).abs() < f64::EPSILON, "delta: {delta}");
}

#[test]
fn completion_weight_delta_ignores_non_todo_to_done_changes() {
    let old_file_content = "\
- [-] task
";
    let new_file_content = "\
- [x] task
";

    let old_items = parse_items(old_file_content);
    let new_items = parse_items(new_file_content);
    let (delta, events) = completion_weight_delta(&old_items, &new_items);

    assert_eq!(events, 0, "events: {events}");
    assert!((delta - 0.0).abs() < f64::EPSILON, "delta: {delta}");
}

#[test]
fn completion_weight_delta_ignores_reorder_of_done_and_todo_tasks() {
    let old_file_content = "\
- [x] done task
- [ ] todo task
";
    let new_file_content = "\
- [ ] todo task
- [x] done task
";

    let old_items = parse_items(old_file_content);
    let new_items = parse_items(new_file_content);
    let (delta, events) = completion_weight_delta(&old_items, &new_items);

    assert_eq!(events, 0, "events: {events}");
    assert!((delta - 0.0).abs() < f64::EPSILON, "delta: {delta}");
}

#[test]
fn completion_weight_delta_ignores_done_task_rename() {
    let old_file_content = "\
- [x] old name
";
    let new_file_content = "\
- [x] new name
";

    let old_items = parse_items(old_file_content);
    let new_items = parse_items(new_file_content);
    let (delta, events) = completion_weight_delta(&old_items, &new_items);

    assert_eq!(events, 0, "events: {events}");
    assert!((delta - 0.0).abs() < f64::EPSILON, "delta: {delta}");
}

#[test]
fn completion_weight_delta_counts_todo_to_done_even_when_another_node_reopens() {
    let old_file_content = "\
- [ ] task a
- [x] task b
";
    let new_file_content = "\
- [x] task a
- [ ] task b
";

    let old_items = parse_items(old_file_content);
    let new_items = parse_items(new_file_content);
    let (delta, events) = completion_weight_delta(&old_items, &new_items);

    assert_eq!(events, 1, "events: {events}");
    assert!((delta - 1.0).abs() < f64::EPSILON, "delta: {delta}");
}

#[test]
fn completion_weight_delta_uses_fallback_matching_when_ancestor_title_changes() {
    let old_file_content = "\
- [ ] grand old
  - [ ] parent
    - [ ] leaf
";
    let new_file_content = "\
- [ ] grand new
  - [ ] parent
    - [x] leaf
";

    let old_items = parse_items(old_file_content);
    let new_items = parse_items(new_file_content);
    let (delta, events) = completion_weight_delta(&old_items, &new_items);

    assert_eq!(events, 1, "events: {events}");
    assert!((delta - (1.0 / 3.0)).abs() < f64::EPSILON, "delta: {delta}");
}

#[test]
fn render_todo_done_data_outputs_table_of_counts_only() {
    let plot = TodoDonePlot {
        milestone_name: "alpha".to_string(),
        points: vec![
            TodoDonePlotPoint {
                date: "2026-07-10".to_string(),
                total_weight: 2.0,
                done_weight: 0.0,
                total_count: 2,
                done_count: 0,
            },
            TodoDonePlotPoint {
                date: "2026-07-11".to_string(),
                total_weight: 2.0,
                done_weight: 1.0,
                total_count: 2,
                done_count: 1,
            },
        ],
    };

    let out = render_todo_done_data(&plot);

    assert!(out.contains("Milestone: alpha"), "out: {out:?}");
    // Header + rows, task counts only — no weights, no trend line data.
    assert!(out.contains("Date"), "out: {out:?}");
    assert!(out.contains("Total"), "out: {out:?}");
    assert!(out.contains("Done"), "out: {out:?}");
    assert!(!out.contains("trend"), "out: {out:?}");
    assert!(!out.contains("2.00"), "out (weight leaked): {out:?}");

    let row1 = out
        .lines()
        .find(|line| line.contains("2026-07-10"))
        .unwrap_or_else(|| panic!("missing row for 2026-07-10, out: {out:?}"));
    assert!(row1.contains('2') && row1.contains('0'), "row1: {row1:?}");

    let row2 = out
        .lines()
        .find(|line| line.contains("2026-07-11"))
        .unwrap_or_else(|| panic!("missing row for 2026-07-11, out: {out:?}"));
    assert!(row2.contains('2') && row2.contains('1'), "row2: {row2:?}");
}

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
        slope: 0.0,
        intercept: 10.0,
    };
    let done_trend = LinearTrend {
        slope: 1.0,
        intercept: 0.0,
    };
    // Anchor is day 0; "today" is day 0 too. Lines cross at x = 10.
    let eta = compute_eta(Some(total_trend), Some(done_trend), Some(0), Some(0))
        .expect("expected an ETA");
    assert_eq!(eta.unix_days, 10);
}

#[test]
fn compute_eta_is_none_when_trend_lines_are_parallel() {
    let total_trend = LinearTrend {
        slope: 1.0,
        intercept: 10.0,
    };
    let done_trend = LinearTrend {
        slope: 1.0,
        intercept: 0.0,
    };
    assert!(compute_eta(Some(total_trend), Some(done_trend), Some(0), Some(0)).is_none());
}

#[test]
fn compute_eta_is_none_when_intersection_is_in_the_past() {
    let total_trend = LinearTrend {
        slope: 0.0,
        intercept: 10.0,
    };
    let done_trend = LinearTrend {
        slope: 1.0,
        intercept: 0.0,
    };
    // Intersection is at x = 10 (relative to anchor), but "today" is day 20,
    // i.e. the crossing already happened in the past.
    assert!(compute_eta(Some(total_trend), Some(done_trend), Some(0), Some(20)).is_none());
}

#[test]
fn compute_eta_is_none_without_both_trends_or_anchor() {
    let total_trend = LinearTrend {
        slope: 0.0,
        intercept: 10.0,
    };
    assert!(compute_eta(Some(total_trend), None, Some(0), Some(0)).is_none());
    let done_trend = LinearTrend {
        slope: 1.0,
        intercept: 0.0,
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

#[test]
fn render_when_line_pads_span_before_milestone_name() {
    // unix_days = 10; "today" = 3, so 7 days remain -> "1 week".
    let eta = EtaEstimate { unix_days: 10 };
    let out = render_when_line("Release of MVP :)", Some(eta), Some(3));
    assert_eq!(out, "1 week    Release of MVP :)\n");
}

#[test]
fn render_when_line_shows_unknown_when_eta_is_unresolved() {
    let out = render_when_line("Release of MVP :)", None, Some(0));
    assert_eq!(out, "unknown   Release of MVP :)\n");
}

#[test]
fn future_milestone_names_skips_already_reached_milestones() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task a
#MILESTONE: alpha
- [ ] task b
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let names = future_milestone_names(dir.path());

    assert_eq!(names, vec!["beta".to_string()]);
}

#[test]
fn render_velocity_text_shows_both_metrics_right_aligned() {
    let estimate = VelocityEstimate {
        velocity_per_week: Some(1.0),
        creep_per_week: Some(0.5),
    };
    let out = render_velocity_text(estimate);
    assert_eq!(
        out,
        "velocity: weight/week   1.00\ncreep:    weight/week   0.50\n"
    );
}

#[test]
fn render_velocity_text_shows_unknown_per_metric_independently() {
    let estimate = VelocityEstimate {
        velocity_per_week: None,
        creep_per_week: Some(2.0),
    };
    let out = render_velocity_text(estimate);
    assert_eq!(out, "velocity: unknown\ncreep:    weight/week   2.00\n");
}

#[test]
fn render_velocity_text_shows_unknown_for_both_when_neither_resolves() {
    let estimate = VelocityEstimate {
        velocity_per_week: None,
        creep_per_week: None,
    };
    let out = render_velocity_text(estimate);
    assert_eq!(out, "velocity: unknown\ncreep:    unknown\n");
}

#[test]
fn estimate_velocity_with_window_errors_when_not_a_git_repository() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let err = estimate_velocity_with_window(dir.path(), 90).unwrap_err();
    assert!(err.contains("requires a git repository"));
}

#[test]
fn estimate_velocity_with_window_reports_unresolved_metrics_for_zero_window() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap();

    let estimate = estimate_velocity_with_window(dir.path(), 0).unwrap();
    assert_eq!(
        estimate,
        VelocityEstimate {
            velocity_per_week: None,
            creep_per_week: None,
        }
    );
}
