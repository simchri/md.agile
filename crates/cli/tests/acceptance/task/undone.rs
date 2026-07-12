use crate::helpers::run_agile;
use std::fs;
use tempfile::tempdir;

#[test]
fn task_undone_reverts_the_most_recently_done_top_level_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] first done task
- [x] second done task
- [ ] still open
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // Address `1` counts from the end: the most recently completed
    // top-level task in document order is "second done task".
    let out = run_agile(dir.path(), &["task", "undone", "1"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    let expected = "\
- [x] first done task
- [ ] second done task
- [ ] still open
";
    assert_eq!(new_content, expected);
}

#[test]
fn task_undone_address_2_reverts_the_second_most_recently_done_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] first done task
- [x] second done task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "undone", "2"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    let expected = "\
- [ ] first done task
- [x] second done task
";
    assert_eq!(new_content, expected);
}

#[test]
fn task_undone_reverts_a_done_subtask_via_dotted_address() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] parent task
  - [x] finished subtask
  - [ ] open subtask
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // The parent task itself is done, so `1` counts it as the 1st done
    // top-level task from the end; `.1` then descends into its 1st child,
    // regardless of that child's own status.
    let out = run_agile(dir.path(), &["task", "undone", "1.1"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    let expected = "\
- [x] parent task
  - [ ] finished subtask
  - [ ] open subtask
";
    assert_eq!(new_content, expected);
}

#[test]
fn task_undone_refuses_a_task_that_is_still_todo() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // Reverting it once succeeds...
    let out = run_agile(dir.path(), &["task", "undone", "1"]);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);

    // ...but doing it again fails: there's no done top-level task left.
    let out = run_agile(dir.path(), &["task", "undone", "1"]);
    assert!(!out.status.success());
}

#[test]
fn task_undone_invalid_address_exits_nonzero() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "undone", "5"]);
    assert!(!out.status.success());
}
