use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_ranked_task_done_out_of_order() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] make app more responsive
  - [ ] 1. add performance UI test
  - [x] 2. refactor signals
  - [ ] 4. document learnings
  - [ ] 3. run UI tests
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E015"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_ranked_tasks_completed_in_sequence() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] make app more responsive
  - [x] 1. add performance UI test
  - [x] 2. refactor signals
  - [ ] 3. run UI tests
  - [ ] 4. document learnings
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
