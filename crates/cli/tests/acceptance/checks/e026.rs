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
fn default_is_valid_entry_true_treats_the_placeholder_itself_as_filled() {
    // The flag only reclassifies the resolved placeholder as a legitimate,
    // deliberately chosen value -- it does not disable the check outright
    // (see the blank-value test below).
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
fn default_is_valid_entry_true_still_flags_a_truly_blank_value() {
    // `default_is_valid_entry` reclassifies the placeholder, not the check
    // itself -- a field left entirely blank (no value at all after the
    // label) is never a legitimate answer and must still be flagged.
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
  __Gender:__ 
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E026"), "stdout: {stdout:?}");
    assert!(stdout.contains("Gender"), "stdout: {stdout:?}");
}

#[test]
fn a_truly_blank_value_is_flagged_even_without_default_is_valid_entry() {
    let dir = tempdir().unwrap();
    write_onboarding_config(dir.path());
    let file_content = "\
- [x] new colleague: John #onboarding
  __First Name:__ 
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
fn two_fields_sharing_the_same_label_within_one_form_is_rejected_at_config_load() {
    // Field labels must be unique within a single form (see
    // `config::tests::form_with_two_different_fields_sharing_the_same_explicit_label_is_rejected`):
    // this previously used to be a silently-deduped "aliasing" feature --
    // a single physical field line satisfying two distinct `[Field.X]`
    // entries that happen to share a label -- which this check rule relied
    // on to avoid reporting the same unfilled line twice. Now that the
    // config itself refuses to load such a form, that scenario can no
    // longer reach this check at all.
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

    assert_ne!(out.status.code(), Some(0), "expected non-zero exit");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("First Name"), "stderr: {stderr:?}");
}

fn write_multiline_bio_config(dir: &std::path::Path) {
    let config = "\
[Field.bio]
label = \"Bio\"
multiline = true
[Form.personal_info]
fields = [\"bio\"]
[Properties.onboarding]
form = \"personal_info\"
";
    fs::write(dir.join("mdagile.toml"), config).unwrap();
}

#[test]
fn flags_a_done_task_whose_multiline_field_has_no_content_before_the_closing_marker() {
    let dir = tempdir().unwrap();
    write_multiline_bio_config(dir.path());
    let file_content = "\
- [x] new colleague: John #onboarding
  __Bio:__
  ...
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E026"), "stdout: {stdout:?}");
    assert!(stdout.contains("Bio"), "stdout: {stdout:?}");
}

#[test]
fn flags_a_done_task_whose_multiline_field_has_only_blank_lines_before_the_closing_marker() {
    let dir = tempdir().unwrap();
    write_multiline_bio_config(dir.path());
    let file_content = "\
- [x] new colleague: John #onboarding
  __Bio:__

  ...
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E026"), "stdout: {stdout:?}");
    assert!(stdout.contains("Bio"), "stdout: {stdout:?}");
}

#[test]
fn flags_a_done_task_whose_multiline_field_has_nothing_after_the_label_and_no_closing_marker() {
    // No closing `...` at all, and the field is the last line in the
    // node's body -- the value is still the empty string, same as the
    // other "no content" variants above.
    let dir = tempdir().unwrap();
    write_multiline_bio_config(dir.path());
    let file_content = "\
- [x] new colleague: John #onboarding
  __Bio:__
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E026"), "stdout: {stdout:?}");
    assert!(stdout.contains("Bio"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_a_done_task_whose_multiline_field_has_real_content() {
    let dir = tempdir().unwrap();
    write_multiline_bio_config(dir.path());
    let file_content = "\
- [x] new colleague: John #onboarding
  __Bio:__
  Grew up in Portland, now leads the platform team.
  ...
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
fn does_not_flag_an_open_task_with_an_empty_multiline_field() {
    // Same "only matters at completion" rule as single-line fields.
    let dir = tempdir().unwrap();
    write_multiline_bio_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  ...
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
