use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn unknown_outcome_satisfies_required_title_but_still_reports_e022() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature]
subtasks = [\"check #review:...\"]
[Properties.review]
[Properties.review.passed]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] #feature
  - [ ] \"check #review:unknown\"
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout.matches("error[").count(), 1, "stdout: {stdout}");
    assert!(stdout.contains("error[E022]"), "stdout: {stdout}");
}

#[test]
fn flags_a_resolved_branch_that_is_not_declared_in_config() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
[Properties.review.passed]
[Properties.review.failed]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] perform #review:bogus
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E022"), "stdout: {stdout:?}");
    assert!(stdout.contains("bogus"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_a_resolved_branch_that_is_declared_in_config() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
[Properties.review.passed]
[Properties.review.failed]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] perform #review:passed
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E022"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_an_entirely_undefined_property_resolved_branch() {
    // Left to E008 "Undefined Property" instead.
    let dir = tempdir().unwrap();
    let content = "\
- [ ] perform #review:passed
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E008"), "stdout: {stdout:?}");
    assert!(!stdout.contains("E022"), "stdout: {stdout:?}");
}
