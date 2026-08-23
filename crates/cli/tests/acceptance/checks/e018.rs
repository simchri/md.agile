use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_bare_milestone_tag_with_no_name() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task one
#MILESTONE
- [ ] task two
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E018"), "stdout: {stdout:?}");
}

#[test]
fn flags_milestone_tag_with_colon_but_no_name() {
    let dir = tempdir().unwrap();
    let file_content = "\
#MILESTONE:
- [ ] task
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E018"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_milestone_with_a_name() {
    let dir = tempdir().unwrap();
    let file_content = "\
#MILESTONE: Release of MVP
- [ ] task
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
