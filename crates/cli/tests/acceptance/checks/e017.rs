use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_duplicate_milestone_name_within_a_single_file() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task one
#MILESTONE: Release of MVP
- [ ] task two
#MILESTONE: Release of MVP
- [ ] task three
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E017"), "stdout: {stdout:?}");
}

#[test]
fn flags_duplicate_milestone_name_across_files() {
    // Milestone names must be unique across the whole project (README.md),
    // not just within a single file.
    let dir = tempdir().unwrap();
    let file_a_content = "\
#MILESTONE: Release of MVP
- [ ] task in file a
";
    fs::write(dir.path().join("a.agile.md"), file_a_content).unwrap();
    let file_b_content = "\
#MILESTONE: Release of MVP
- [ ] task in file b
";
    fs::write(dir.path().join("b.agile.md"), file_b_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E017"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_distinct_milestone_names() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task one
#MILESTONE: Alpha
- [ ] task two
#MILESTONE: Beta
- [ ] task three
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
