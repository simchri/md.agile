use crate::helpers::run_agile;
use std::fs;
use tempfile::tempdir;

#[test]
fn list_prints_active_tasks() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] first task
- [ ] second task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["list"]);

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] first task"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] second task"), "stdout: {stdout:?}");
}

#[test]
fn list_excludes_done_and_cancelled_by_default() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done task
- [-] cancelled task
- [ ] active task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["list"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !stdout.contains("[x]"),
        "done task should be excluded: {stdout:?}"
    );
    assert!(
        !stdout.contains("[-]"),
        "cancelled task should be excluded: {stdout:?}"
    );
    assert!(stdout.contains("[ ] active task"), "stdout: {stdout:?}");
}

#[test]
fn list_all_includes_done_and_cancelled() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done task
- [-] cancelled task
- [ ] active task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["list", "--all"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[x] done task"), "stdout: {stdout:?}");
    assert!(stdout.contains("[-] cancelled task"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] active task"), "stdout: {stdout:?}");
}

#[test]
fn list_includes_subtasks_in_output() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
  - [x] subtask done
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["list"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] parent task"), "stdout: {stdout:?}");
    assert!(stdout.contains("  [ ] subtask one"), "stdout: {stdout:?}");
    assert!(stdout.contains("  [x] subtask done"), "stdout: {stdout:?}");
}

#[test]
fn list_with_next_limit_returns_first_n_tasks() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] task one
- [ ] task two
- [ ] task three
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["list", "-n", "2"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] task one"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] task two"), "stdout: {stdout:?}");
    assert!(!stdout.contains("task three"), "stdout: {stdout:?}");
}

#[test]
fn list_with_last_limit_returns_last_n_tasks() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] task one
- [ ] task two
- [ ] task three
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["list", "--last", "1"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("task one"), "stdout: {stdout:?}");
    assert!(!stdout.contains("task two"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] task three"), "stdout: {stdout:?}");
}

#[test]
fn list_next_flag_takes_precedence_over_last() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] task one
- [ ] task two
- [ ] task three
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // --next 1 and --last 1 both set: --next wins, so we get the first task
    let out = run_agile(dir.path(), &["list", "-n", "1", "--last", "1"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] task one"), "stdout: {stdout:?}");
    assert!(!stdout.contains("task three"), "stdout: {stdout:?}");
}

#[test]
fn list_empty_project_exits_zero_with_no_output() {
    let dir = tempdir().unwrap();

    let out = run_agile(dir.path(), &["list"]);

    assert!(out.status.success());
    assert!(
        out.stdout.is_empty(),
        "expected no output: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}
