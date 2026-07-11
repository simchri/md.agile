use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_duplicate_rank_among_siblings() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] make app more responsive
  - [ ] 1. add performance UI test
  - [ ] 1. refactor signals
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E014"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_distinct_ranks_out_of_textual_order() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] make app more responsive
  - [ ] 1. add performance UI test
  - [ ] 2. refactor signals
  - [ ] 4. document learnings
  - [ ] 3. run UI tests
  - [ ] discuss further steps
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
