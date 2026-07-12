use crate::helpers::run_agile;
use std::fs;
use tempfile::tempdir;

#[test]
fn task_done_marks_top_level_task_complete_in_place() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] first task
- [ ] second task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "1"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    let expected = "\
- [x] first task
- [ ] second task
";
    assert_eq!(new_content, expected);
}

#[test]
fn task_done_marks_specific_subtask_via_dotted_address() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
  - [ ] subtask two
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "1.2"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    let expected = "\
- [ ] parent task
  - [ ] subtask one
  - [x] subtask two
";
    assert_eq!(new_content, expected);
}

#[test]
fn task_done_refuses_when_a_required_child_is_incomplete() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "1"]);

    assert!(!out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E004"), "stdout: {stdout:?}");
    // File must be left untouched.
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    assert_eq!(new_content, content);
}

#[test]
fn task_done_refuses_when_a_required_subtask_is_missing() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.needs_review]
subtasks = [\"code review\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] #needs_review parent task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "1"]);

    assert!(!out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E010"), "stdout: {stdout:?}");
}

#[test]
fn task_done_invalid_address_exits_nonzero() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "5"]);

    assert!(!out.status.success());
}

#[test]
fn task_done_on_already_done_task_exits_nonzero() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] already done
- [ ] still open
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // Address 1 selects the first *incomplete* top-level task ("still
    // open"), so marking it done twice in a row should fail the second time.
    let out = run_agile(dir.path(), &["task", "done", "1"]);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);

    let out = run_agile(dir.path(), &["task", "done", "1"]);
    assert!(!out.status.success());
}
