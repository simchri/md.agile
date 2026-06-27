use crate::helpers::run_agile;
use std::fs;
use tempfile::tempdir;

#[test]
fn task_next_prints_first_active_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done task
- [ ] first active task
- [ ] second active task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "next"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("[ ] first active task"),
        "stdout: {stdout:?}"
    );
    assert!(
        !stdout.contains("second active"),
        "should only show first: {stdout:?}"
    );
}

#[test]
fn task_next_includes_subtree_of_next_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
  - [x] subtask done
- [ ] other task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "next"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] parent task"), "stdout: {stdout:?}");
    assert!(stdout.contains("  [ ] subtask one"), "stdout: {stdout:?}");
    assert!(stdout.contains("  [x] subtask done"), "stdout: {stdout:?}");
    assert!(!stdout.contains("other task"), "stdout: {stdout:?}");
}

#[test]
fn task_next_empty_project_exits_zero_with_no_output() {
    let dir = tempdir().unwrap();

    let out = run_agile(dir.path(), &["task", "next"]);

    assert!(out.status.success());
    assert!(
        out.stdout.is_empty(),
        "expected no output: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn tasks_alias_works_for_next() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] the task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["tasks", "next"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] the task"), "stdout: {stdout:?}");
}
