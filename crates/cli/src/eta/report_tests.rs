use super::*;
use std::fs;
use tempfile::tempdir;

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
        velocity_wtpw: Some(1.0),
        creep_wtpw: Some(0.5),
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
        velocity_wtpw: None,
        creep_wtpw: Some(2.0),
    };
    let out = render_velocity_text(estimate);
    assert_eq!(out, "velocity: unknown\ncreep:    weight/week   2.00\n");
}

#[test]
fn render_velocity_text_shows_unknown_for_both_when_neither_resolves() {
    let estimate = VelocityEstimate {
        velocity_wtpw: None,
        creep_wtpw: None,
    };
    let out = render_velocity_text(estimate);
    assert_eq!(out, "velocity: unknown\ncreep:    unknown\n");
}

#[test]
fn detail_report_errors_when_not_a_git_repository() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let err = build_when_detail_report(dir.path(), 1, TrendFitAlgorithm::ExponentialDecay, None)
        .unwrap_err();
    assert!(err.contains("git repository"));
}

#[test]
fn detail_report_errors_for_out_of_range_rank() {
    let dir = tempdir().unwrap();
    let _ = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let err = build_when_detail_report(dir.path(), 5, TrendFitAlgorithm::ExponentialDecay, None)
        .unwrap_err();
    assert_eq!(err, "milestone rank 5 does not exist");
}
