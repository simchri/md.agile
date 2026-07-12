use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_task_with_no_title_text() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] 
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E016"), "stdout: {stdout:?}");
}

#[test]
fn flags_subtask_with_no_title_text() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task
  - [ ] 
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E016"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_task_with_title_text() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a real task
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
