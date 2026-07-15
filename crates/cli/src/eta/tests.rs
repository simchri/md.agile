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
    let delta = completion_weight_delta(&old_items, &new_items);

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
    let delta = completion_weight_delta(&old_items, &new_items);

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
    let delta = completion_weight_delta(&old_items, &new_items);

    assert!((delta - 0.0).abs() < f64::EPSILON, "delta: {delta}");
}
