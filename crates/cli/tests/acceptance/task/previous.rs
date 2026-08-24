use crate::helpers::run_agile;
use std::fs;
use tempfile::tempdir;

#[test]
fn task_previous_shows_the_last_fully_done_top_level_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] first done task
- [x] second done task
- [ ] still open task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "previous"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("second done task"), "stdout: {stdout:?}");
    assert!(
        !stdout.contains("first done task"),
        "should only show the last closed task: stdout: {stdout:?}"
    );
    assert!(!stdout.contains("still open task"), "stdout: {stdout:?}");
}

#[test]
fn task_previous_counts_a_partially_completed_top_level_task_as_a_candidate() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] fully done task
- [ ] partially done task
  - [x] finished subtask
  - [ ] open subtask
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // "partially done task" is the most recent one with any closed work in
    // it (its "finished subtask" child), so it's address 1, not "fully done
    // task".
    let out = run_agile(dir.path(), &["task", "previous"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("[ ] partially done task"),
        "stdout: {stdout:?}"
    );
    assert!(stdout.contains("finished subtask"), "stdout: {stdout:?}");
    assert!(stdout.contains("open subtask"), "stdout: {stdout:?}");
    assert!(!stdout.contains("fully done task"), "stdout: {stdout:?}");
}

#[test]
fn task_previous_prints_full_subtree_of_the_selected_top_level_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] closed parent
  - [x] closed child one
  - [x] closed child two
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "previous"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[x] closed parent"), "stdout: {stdout:?}");
    assert!(stdout.contains("closed child one"), "stdout: {stdout:?}");
    assert!(stdout.contains("closed child two"), "stdout: {stdout:?}");
}

#[test]
fn task_previous_treats_cancelled_as_closed() {
    let dir = tempdir().unwrap();
    let content = "\
- [-] cancelled task
- [ ] still open task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "previous"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("cancelled task"), "stdout: {stdout:?}");
}

#[test]
fn task_previous_numbers_in_reverse_priority_order() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] alpha done
- [x] beta done
- [x] gamma done
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out1 = run_agile(dir.path(), &["task", "previous", "1"]);
    let out2 = run_agile(dir.path(), &["task", "previous", "2"]);
    let out3 = run_agile(dir.path(), &["task", "previous", "3"]);

    assert!(out1.status.success());
    assert!(out2.status.success());
    assert!(out3.status.success());
    assert!(String::from_utf8(out1.stdout).unwrap().contains("gamma"));
    assert!(String::from_utf8(out2.stdout).unwrap().contains("beta"));
    assert!(String::from_utf8(out3.stdout).unwrap().contains("alpha"));
}

#[test]
fn task_previous_empty_project_exits_zero_with_no_output() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] nothing closed yet
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "previous"]);

    assert!(out.status.success());
    assert!(
        out.stdout.is_empty(),
        "expected no output: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn task_previous_explicit_out_of_range_address_is_an_error() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] only done task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "previous", "2"]);

    assert!(!out.status.success());
}

#[test]
fn prev_alias_works_for_previous() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] the task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "prev"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("the task"), "stdout: {stdout:?}");
}

#[test]
fn task_previous_dotted_address_reaches_a_specific_child() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] closed parent
  - [x] closed child one
  - [x] closed child two
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "previous", "1.2"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("[x] closed child two"),
        "stdout: {stdout:?}"
    );
    assert!(!stdout.contains("closed child one"), "stdout: {stdout:?}");
}
