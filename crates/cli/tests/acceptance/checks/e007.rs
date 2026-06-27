use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_uppercase_x_in_status_box() {
    let dir = tempdir().unwrap();
    let content = "\
- [X] task title
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E007"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_lowercase_x() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] task title
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
