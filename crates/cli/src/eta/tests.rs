use super::*;
use std::path::PathBuf;

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
