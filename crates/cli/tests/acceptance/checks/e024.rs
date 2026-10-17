//! Acceptance tests for E024 "Missing Required Field" (see
//! `crate::rules::missing_required_field`): a task/subtask whose property
//! attaches a `[Form.X]` is flagged when its body is missing one of that
//! form's `__Label:__ value` field lines entirely.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

fn write_onboarding_config(dir: &std::path::Path) {
    let config = "\
[Field.first_name]
label = \"First Name\"
[Field.email]
label = \"Email\"
[Form.personal_info]
fields = [\"first_name\", \"email\"]
[Properties.onboarding]
form = \"personal_info\"
";
    fs::write(dir.join("mdagile.toml"), config).unwrap();
}

#[test]
fn flags_a_form_field_line_missing_entirely() {
    let dir = tempdir().unwrap();
    write_onboarding_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __First Name:__ John
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E024"), "stdout: {stdout:?}");
    assert!(stdout.contains("Email"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_when_every_form_field_line_is_present() {
    let dir = tempdir().unwrap();
    write_onboarding_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __First Name:__ John
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
fn does_not_flag_a_task_without_the_property_at_all() {
    let dir = tempdir().unwrap();
    write_onboarding_config(dir.path());
    let file_content = "\
- [ ] a plain task with no form attached
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
fn a_malformed_field_line_is_reported_as_e025_not_also_as_missing() {
    // A label with a malformed line (missing the space before its value) is
    // excluded from E024's "missing" computation -- E025 already reports
    // the real problem with a more specific message, so reporting both
    // would be redundant.
    let dir = tempdir().unwrap();
    write_onboarding_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __First Name:__John
  __Email:__ john@example.com
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E025"), "stdout: {stdout:?}");
    assert!(!stdout.contains("E024"), "stdout: {stdout:?}");
}
