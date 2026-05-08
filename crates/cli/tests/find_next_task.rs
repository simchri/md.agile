use mdagile::cli::subcommands::default::find_next_task;
use std::fs;
use tempfile::tempdir;

#[test]
fn returns_none_when_no_files() {
    let dir = tempdir().unwrap();
    assert_eq!(find_next_task(dir.path()), None);
}

#[test]
fn returns_none_when_all_tasks_done() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.agile.md"), "- [x] done task\n").unwrap();
    assert_eq!(find_next_task(dir.path()), None);
}

#[test]
fn returns_none_when_all_tasks_cancelled() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.agile.md"), "- [-] cancelled task\n").unwrap();
    assert_eq!(find_next_task(dir.path()), None);
}

#[test]
fn returns_path_and_line_of_active_task() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("a.agile.md");
    fs::write(&path, "- [ ] active task\n").unwrap();
    assert_eq!(find_next_task(dir.path()), Some((path, 1)));
}

#[test]
fn skips_all_done_file_and_returns_next() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.agile.md"), "- [x] done\n").unwrap();
    let b_path = dir.path().join("b.agile.md");
    fs::write(&b_path, "- [ ] active\n").unwrap();
    assert_eq!(find_next_task(dir.path()), Some((b_path, 1)));
}

#[test]
fn returns_line_of_first_active_task_after_other_content() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("a.agile.md");
    let input = "\
# heading

- [x] done
- [ ] active task
";
    fs::write(&path, input).unwrap();
    assert_eq!(find_next_task(dir.path()), Some((path, 4)));
}

#[test]
fn skips_subtasks_when_choosing_line() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("a.agile.md");
    let input = "\
- [x] done parent
  - [ ] subtask under done
- [ ] active top level
";
    fs::write(&path, input).unwrap();
    assert_eq!(find_next_task(dir.path()), Some((path, 3)));
}

#[test]
fn returns_highest_priority_file_first() {
    let dir = tempdir().unwrap();
    let a_path = dir.path().join("a.agile.md");
    fs::write(&a_path, "- [ ] first priority\n").unwrap();
    fs::write(dir.path().join("b.agile.md"), "- [ ] second priority\n").unwrap();
    assert_eq!(find_next_task(dir.path()), Some((a_path, 1)));
}
