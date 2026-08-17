use super::*;
use crate::parser::{self, FileItem};
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
fn estimate_velocity_with_window_errors_when_not_a_git_repository() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let err = estimate_velocity_with_window(dir.path(), 1, Some(90)).unwrap_err();
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

    let estimate = estimate_velocity_with_window(dir.path(), 1, Some(0)).unwrap();
    assert_eq!(
        estimate,
        VelocityEstimate {
            velocity_wtpw: None,
            creep_wtpw: None,
        }
    );
}
