//! Acceptance tests for E026 "Field Left Unfilled" (see
//! `crate::rules::field_left_unfilled`): a present form field line whose
//! value still equals its resolved "unfilled placeholder" (its configured
//! `default`, or the plain `___` when it has none).
//!
//! Unlike E024/E025, this only matters at the moment a task is (about to
//! be) marked done: `agile check`'s full-project scan only flags an
//! already-`[x]` task (retroactive detection, mirroring E004's pattern),
//! while `agile task done` additionally blocks completion itself via the
//! same underlying rule (see `task/done.rs`'s
//! `task_done_refuses_when_a_required_field_is_left_unfilled`).

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

fn write_onboarding_config(dir: &std::path::Path) {
    let config = "\
[Field.first_name]
label = \"First Name\"
[Field.gender]
label = \"Gender\"
default = \"(don't specify)\"
[Form.personal_info]
fields = [\"first_name\", \"gender\"]
[Properties.onboarding]
form = \"personal_info\"
";
    fs::write(dir.join("mdagile.toml"), config).unwrap();
}

#[test]
fn flags_a_done_task_whose_field_still_has_the_plain_underscore_placeholder() {
    let dir = tempdir().unwrap();
    write_onboarding_config(dir.path());
    let file_content = "\
- [x] new colleague: John #onboarding
  __First Name:__ ___
  __Gender:__ male
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E026"), "stdout: {stdout:?}");
    assert!(stdout.contains("First Name"), "stdout: {stdout:?}");
}

#[test]
fn flags_a_done_task_whose_field_still_has_its_custom_default() {
    // "Gender" has a configured custom default of "(don't specify)" -- a
    // field that still carries exactly that resolved placeholder counts as
    // unfilled too, same as the plain `___` case.
    let dir = tempdir().unwrap();
    write_onboarding_config(dir.path());
    let file_content = "\
- [x] new colleague: John #onboarding
  __First Name:__ John
  __Gender:__ (don't specify)
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E026"), "stdout: {stdout:?}");
    assert!(stdout.contains("Gender"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_an_open_task_left_at_the_placeholder() {
    // A form field is allowed to sit unfilled for as long as the task stays
    // open -- this only matters at the moment of completion.
    let dir = tempdir().unwrap();
    write_onboarding_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __First Name:__ ___
  __Gender:__ male
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
fn does_not_flag_a_done_task_with_every_field_actually_filled_in() {
    let dir = tempdir().unwrap();
    write_onboarding_config(dir.path());
    let file_content = "\
- [x] new colleague: John #onboarding
  __First Name:__ John
  __Gender:__ male
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
fn default_is_valid_entry_true_suppresses_the_check_for_that_field() {
    let dir = tempdir().unwrap();
    let config = "\
[Field.first_name]
label = \"First Name\"
[Field.gender]
label = \"Gender\"
default = \"(don't specify)\"
default_is_valid_entry = true
[Form.personal_info]
fields = [\"first_name\", \"gender\"]
[Properties.onboarding]
form = \"personal_info\"
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [x] new colleague: John #onboarding
  __First Name:__ John
  __Gender:__ (don't specify)
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
fn two_fields_sharing_the_same_label_report_the_unfilled_label_only_once() {
    // Regression test: several `[Field.X]` entries may share one display
    // `label` so a single physical field line satisfies all of them. Left
    // unfilled, that one line must only be reported once.
    let dir = tempdir().unwrap();
    let config = "\
[Field.first_name]
label = \"First Name\"
[Field.first_name_alias]
label = \"First Name\"
[Form.personal_info]
fields = [\"first_name\", \"first_name_alias\"]
[Properties.onboarding]
form = \"personal_info\"
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [x] new colleague: John #onboarding
  __First Name:__ ___
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let occurrences = stdout.matches("First Name").count();
    assert_eq!(occurrences, 1, "stdout: {stdout:?}");
}
