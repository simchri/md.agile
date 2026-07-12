use crate::helpers::run_agile;
use std::fs;
use tempfile::tempdir;

#[test]
fn task_undone_reverts_a_done_subtask_of_an_open_top_level_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [x] mistakenly finished subtask
  - [ ] open subtask
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // ADDRESS uses exactly the same resolution as `agile task done`: `1`
    // selects the 1st still-incomplete top-level task, `.1` its 1st child.
    let out = run_agile(dir.path(), &["task", "undone", "1.1"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    let expected = "\
- [ ] parent task
  - [ ] mistakenly finished subtask
  - [ ] open subtask
";
    assert_eq!(new_content, expected);
}

#[test]
fn task_undone_reverts_a_deeply_nested_done_grandchild() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] child task
    - [x] mistakenly finished grandchild
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "undone", "1.1.1"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    let expected = "\
- [ ] parent task
  - [ ] child task
    - [ ] mistakenly finished grandchild
";
    assert_eq!(new_content, expected);
}

#[test]
fn task_undone_refuses_a_subtask_that_is_still_todo() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] still open subtask
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "undone", "1.1"]);

    assert!(!out.status.success());
    // File must be left untouched.
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    assert_eq!(new_content, content);
}

#[test]
fn task_undone_cannot_reach_an_already_fully_done_top_level_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] already fully done task
- [ ] still open task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // Address `1` selects the 1st *incomplete* top-level task ("still open
    // task"), same as `agile task done` — the already-done top-level task
    // is never counted, so there's no address that reaches it. This is an
    // accepted, intentional limitation: `undone` is for correcting a
    // mistakenly-completed subtask while its parent is still open, not for
    // reopening a whole completed top-level task.
    let out = run_agile(dir.path(), &["task", "undone", "1"]);

    assert!(!out.status.success());
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    assert_eq!(new_content, content);
}

#[test]
fn task_undone_invalid_address_exits_nonzero() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] a task
  - [x] a subtask
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "undone", "5"]);
    assert!(!out.status.success());
}
