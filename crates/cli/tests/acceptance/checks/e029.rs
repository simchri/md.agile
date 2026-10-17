//! Acceptance tests for E029 "Field Value Not Allowed" (see
//! `crate::rules::field_value_not_allowed`): a field configured with a
//! closed set of `[Field.X] values = [...]` must have one of those values
//! if it's been filled in at all -- any other entered value is flagged.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

fn write_gender_field_config(dir: &std::path::Path) {
    let config = "\
[Field.gender]
label = \"Gender\"
default = \"(don't specify)\"
values = [\"male\", \"female\", \"(don't specify)\"]
[Form.personal_info]
fields = [\"gender\"]
[Properties.onboarding]
form = \"personal_info\"
";
    fs::write(dir.join("mdagile.toml"), config).unwrap();
}

#[test]
fn flags_a_value_outside_the_allowed_set() {
    let dir = tempdir().unwrap();
    write_gender_field_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __Gender:__ nonbinary
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E029"), "stdout: {stdout:?}");
    assert!(stdout.contains("Gender"), "stdout: {stdout:?}");
    // The field line is line 2, not the task line (line 1) -- the issue
    // must be anchored at the field itself.
    assert!(stdout.contains(":2:"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_a_value_within_the_allowed_set() {
    let dir = tempdir().unwrap();
    write_gender_field_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __Gender:__ female
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
fn does_not_flag_a_field_still_at_its_unfilled_placeholder() {
    // Still at the default/placeholder is E026's concern, not this rule's.
    let dir = tempdir().unwrap();
    write_gender_field_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __Gender:__ (don't specify)
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E029"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_a_field_with_no_configured_values() {
    let dir = tempdir().unwrap();
    let config = "\
[Field.first_name]
label = \"First Name\"
[Form.personal_info]
fields = [\"first_name\"]
[Properties.onboarding]
form = \"personal_info\"
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] new colleague: John #onboarding
  __First Name:__ anything at all
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
fn a_done_task_is_flagged_the_same_as_an_open_one() {
    // Like E028, this is a validity mistake independent of whether the
    // task is open or done.
    let dir = tempdir().unwrap();
    write_gender_field_config(dir.path());
    let file_content = "\
- [x] new colleague: John #onboarding
  __Gender:__ nonbinary
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E029"), "stdout: {stdout:?}");
}

#[test]
fn a_task_with_no_fields_at_all_is_not_flagged_here() {
    // Absence is E024's ("missing_required_field") concern, not this
    // rule's.
    let dir = tempdir().unwrap();
    write_gender_field_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E029"), "stdout: {stdout:?}");
}
