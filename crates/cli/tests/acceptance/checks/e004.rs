use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_done_parent_with_incomplete_child() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done parent
  - [ ] incomplete child
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E004"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_done_parent_with_all_children_done() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done parent
  - [x] done child
  - [x] another done child
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
