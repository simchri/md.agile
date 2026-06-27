use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_invalid_box_character() {
    let dir = tempdir().unwrap();
    // `?` is not a valid status character
    let content = "\
- [?] task title
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E006"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_valid_box_characters() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] pending
- [x] done
- [-] cancelled
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
