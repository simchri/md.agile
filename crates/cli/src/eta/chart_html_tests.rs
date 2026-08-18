use super::*;
use crate::eta::plot_data::{TodoDonePlot, TodoDonePlotPoint};
use chrono::NaiveDate;
use std::fs;
use tempfile::tempdir;

#[test]
fn sanitize_milestone_slug_lowercases_and_collapses_non_alphanumerics() {
    assert_eq!(
        sanitize_milestone_slug("Release of MVP :)"),
        "release_of_mvp"
    );
}

#[test]
fn sanitize_milestone_slug_trims_leading_and_trailing_underscores() {
    assert_eq!(sanitize_milestone_slug("  --Beta!!  "), "beta");
}

#[test]
fn sanitize_milestone_slug_falls_back_when_nothing_alphanumeric_remains() {
    assert_eq!(sanitize_milestone_slug("!!!"), "milestone");
}

fn sample_plot() -> TodoDonePlot {
    TodoDonePlot {
        milestone_name: "Beta Release".to_string(),
        points: vec![
            TodoDonePlotPoint {
                date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                total_weight_wt: 10.0,
                done_weight_wt: 2.0,
                total_count_t: 10,
                done_count_t: 2,
            },
            TodoDonePlotPoint {
                date: NaiveDate::from_ymd_opt(2026, 1, 8).unwrap(),
                total_weight_wt: 12.0,
                done_weight_wt: 6.0,
                total_count_t: 12,
                done_count_t: 6,
            },
        ],
    }
}

#[test]
fn write_todo_done_plot_html_writes_a_sanitized_filename() {
    let dir = tempdir().unwrap();
    let plot = sample_plot();

    let path = write_todo_done_plot_html(dir.path(), &plot, crate::eta::DEFAULT_EXTRA).unwrap();

    assert_eq!(path, dir.path().join("beta_release-plot.html"));
    assert!(path.exists(), "the html file should have been written");
}

#[test]
fn write_todo_done_plot_html_content_includes_chart_and_report_sections() {
    let dir = tempdir().unwrap();
    let plot = sample_plot();

    let path = write_todo_done_plot_html(dir.path(), &plot, crate::eta::DEFAULT_EXTRA).unwrap();
    let html = fs::read_to_string(path).unwrap();

    assert!(html.contains("<svg"), "should embed an inline SVG chart");
    assert!(
        html.contains("Milestone: Beta Release"),
        "should show the milestone name"
    );
    assert!(
        html.contains("Trend lines"),
        "should include the trend line equations"
    );
    assert!(
        html.contains("total:") && html.contains("done:"),
        "should include the latest stats"
    );
    assert!(html.contains("ETA:"), "should include the ETA text");
}
