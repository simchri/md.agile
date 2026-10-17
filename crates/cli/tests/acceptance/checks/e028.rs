//! Acceptance tests for E028 "Multiline Field Filled Inline" (see
//! `crate::rules::multiline_field_filled_inline`): a field configured as
//! multiline (see `[Field.X] multiline = true`) must open its label on its
//! own line, with the content on the line(s) below it -- writing it inline
//! (`__Label:__ value`) is flagged.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

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
fn flags_a_multiline_field_written_inline() {
    let dir = tempdir().unwrap();
    write_multiline_bio_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __Bio:__ Grew up in Portland.
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E028"), "stdout: {stdout:?}");
    assert!(stdout.contains("Bio"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_a_multiline_field_written_properly_across_lines() {
    let dir = tempdir().unwrap();
    write_multiline_bio_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  Grew up in Portland.
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
fn flags_an_inline_multiline_field_even_when_its_value_is_blank() {
    // `__Bio:__ ` (trailing space, no content) still matches the
    // single-line grammar, not the bare multiline-opening one, so it's
    // flagged too -- this rule only cares which grammar was used, not
    // whether the resulting value happens to be empty.
    let dir = tempdir().unwrap();
    write_multiline_bio_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
  __Bio:__ 
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E028"), "stdout: {stdout:?}");
}

#[test]
fn a_single_line_configured_field_written_inline_is_never_flagged() {
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
  __First Name:__ John
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
fn a_task_with_no_fields_at_all_is_not_flagged_here() {
    // Absence is E024's ("missing_required_field") concern, not this
    // rule's.
    let dir = tempdir().unwrap();
    write_multiline_bio_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E028"), "stdout: {stdout:?}");
}

#[test]
fn a_done_task_is_flagged_the_same_as_an_open_one() {
    // Unlike E026, this is a structural/syntax mistake, independent of
    // whether the task is open or done.
    let dir = tempdir().unwrap();
    write_multiline_bio_config(dir.path());
    let file_content = "\
- [x] new colleague: John #onboarding
  __Bio:__ Grew up in Portland.
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E028"), "stdout: {stdout:?}");
}
