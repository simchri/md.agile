use super::*;
use crate::eta::plot_data::{TodoDonePlot, TodoDonePlotPoint};
use chrono::NaiveDate;

#[test]
fn render_todo_done_data_outputs_table_of_counts_and_weights() {
    let plot = TodoDonePlot {
        milestone_name: "alpha".to_string(),
        points: vec![
            TodoDonePlotPoint {
                date: NaiveDate::from_ymd_opt(2026, 7, 10).unwrap(),
                total_weight_wt: 2.5,
                done_weight_wt: 0.0,
                total_count_t: 3,
                done_count_t: 0,
                total_top_level_t: 2,
                done_top_level_t: 0,
            },
            TodoDonePlotPoint {
                date: NaiveDate::from_ymd_opt(2026, 7, 11).unwrap(),
                total_weight_wt: 2.5,
                done_weight_wt: 1.5,
                total_count_t: 3,
                done_count_t: 2,
                total_top_level_t: 2,
                done_top_level_t: 1,
            },
        ],
    };

    let out = render_todo_done_data(&plot);

    assert!(out.contains("Milestone: alpha"), "out: {out:?}");
    // Header + rows: counts and weights, but no trend line fitting.
    assert!(out.contains("Date"), "out: {out:?}");
    assert!(out.contains("Total"), "out: {out:?}");
    assert!(out.contains("Done"), "out: {out:?}");
    assert!(out.contains("Total Wt"), "out: {out:?}");
    assert!(out.contains("Done Wt"), "out: {out:?}");
    assert!(!out.contains("trend"), "out: {out:?}");

    let row1 = out
        .lines()
        .find(|line| line.contains("2026-07-10"))
        .unwrap_or_else(|| panic!("missing row for 2026-07-10, out: {out:?}"));
    // Total/Done columns show top-level-only counts (2, 0), not the
    // subtask-inclusive total_count_t/done_count_t (3, 0).
    let fields1: Vec<&str> = row1.split_whitespace().collect();
    assert_eq!(
        fields1[1], "2",
        "Total column should be top-level count, not subtask-inclusive: row1: {row1:?}"
    );
    assert_eq!(fields1[2], "0", "row1: {row1:?}");
    assert!(
        row1.contains("2.50") && row1.contains("0.00"),
        "row1: {row1:?}"
    );

    let row2 = out
        .lines()
        .find(|line| line.contains("2026-07-11"))
        .unwrap_or_else(|| panic!("missing row for 2026-07-11, out: {out:?}"));
    // Top-level-only counts (2, 1), not subtask-inclusive counts (3, 2).
    let fields2: Vec<&str> = row2.split_whitespace().collect();
    assert_eq!(
        fields2[1], "2",
        "Total column should be top-level count, not subtask-inclusive: row2: {row2:?}"
    );
    assert_eq!(fields2[2], "1", "row2: {row2:?}");
    assert!(
        row2.contains("2.50") && row2.contains("1.50"),
        "row2: {row2:?}"
    );
}
