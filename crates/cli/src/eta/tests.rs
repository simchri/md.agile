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
fn render_todo_done_plot_legend_uses_2x2_colored_grid() {
    let plot = TodoDonePlot {
        milestone_name: "M1".to_string(),
        points: vec![
            TodoDonePlotPoint {
                date: "2026-01-01".to_string(),
                total_weight: 3.0,
                done_weight: 1.0,
            },
            TodoDonePlotPoint {
                date: "2026-01-02".to_string(),
                total_weight: 2.0,
                done_weight: 1.5,
            },
        ],
    };

    let rendered = render_todo_done_plot(&plot);
    let expected_legend = "\
legend:
\u{1b}[38;2;255;0;0m....\u{1b}[0m total    \u{1b}[38;2;0;255;0m....\u{1b}[0m done
\u{1b}[38;2;255;255;0m....\u{1b}[0m total trend    \u{1b}[38;2;0;255;255m....\u{1b}[0m done trend
";

    assert!(
        rendered.contains(expected_legend),
        "expected legend block:\n{expected_legend}\nrendered:\n{rendered}"
    );
}
