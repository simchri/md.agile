//! Acceptance tests for E025 "Missing Space After Field Label" (see
//! `crate::rules::missing_space_after_field_label`): a `__Label:__value`
//! field line missing the required space before its value.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_a_field_label_missing_the_space_before_its_value() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
  __Email:__john@example.com
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E025"), "stdout: {stdout:?}");
    assert!(stdout.contains("Email"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_a_properly_spaced_field_line() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
  __Email:__ john@example.com
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn this_check_needs_no_config_and_fires_without_a_form_attached() {
    // Purely structural -- no `mdagile.toml` / `[Field.X]` / `[Form.X]` /
    // property involved at all.
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task with no property or config whatsoever
  __Notes:__some text
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E025"), "stdout: {stdout:?}");
}
