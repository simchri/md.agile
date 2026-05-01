use mdagile::cli::subcommands::default::{find_file_with_next_task, find_next_task_line};
use std::fs;
use tempfile::tempdir;

#[test]
fn returns_none_when_no_files() {
    let dir = tempdir().unwrap();
    assert_eq!(find_file_with_next_task(dir.path()), None);
}

#[test]
fn returns_none_when_all_tasks_done() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.agile.md"), "- [x] done task\n").unwrap();
    assert_eq!(find_file_with_next_task(dir.path()), None);
}

#[test]
fn returns_none_when_all_tasks_cancelled() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.agile.md"), "- [-] cancelled task\n").unwrap();
    assert_eq!(find_file_with_next_task(dir.path()), None);
}

#[test]
fn returns_file_containing_active_task() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("a.agile.md");
    fs::write(&path, "- [ ] active task\n").unwrap();
    assert_eq!(find_file_with_next_task(dir.path()), Some(path));
}

#[test]
fn skips_all_done_file_and_returns_next() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.agile.md"), "- [x] done\n").unwrap();
    let b_path = dir.path().join("b.agile.md");
    fs::write(&b_path, "- [ ] active\n").unwrap();
    assert_eq!(find_file_with_next_task(dir.path()), Some(b_path));
}

// ── find_next_task_line ───────────────────────────────────────────────────────

#[test]
fn line_number_of_task_at_first_line() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("a.agile.md");
    fs::write(&path, "- [ ] first task\n").unwrap();
    assert_eq!(find_next_task_line(&path), Some(1));
}

#[test]
fn line_number_of_task_after_other_content() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("a.agile.md");
    let input = "\
# heading

- [x] done
- [ ] active task
";
    fs::write(&path, input).unwrap();
    assert_eq!(find_next_task_line(&path), Some(4));
}

#[test]
fn line_number_returns_none_when_no_active_tasks() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("a.agile.md");
    fs::write(&path, "- [x] done\n- [-] cancelled\n").unwrap();
    assert_eq!(find_next_task_line(&path), None);
}

#[test]
fn line_number_skips_subtasks() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("a.agile.md");
    let input = "\
- [x] done parent
  - [ ] subtask under done
- [ ] active top level
";
    fs::write(&path, input).unwrap();
    assert_eq!(find_next_task_line(&path), Some(3));
}

// ── find_file_with_next_task ──────────────────────────────────────────────────

#[test]
fn returns_highest_priority_file_first() {
    let dir = tempdir().unwrap();
    let a_path = dir.path().join("a.agile.md");
    fs::write(&a_path, "- [ ] first priority\n").unwrap();
    fs::write(dir.path().join("b.agile.md"), "- [ ] second priority\n").unwrap();
    assert_eq!(find_file_with_next_task(dir.path()), Some(a_path));
}
