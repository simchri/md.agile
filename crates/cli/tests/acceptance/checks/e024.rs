//! Acceptance tests for E024 "Missing Required Field" (see
//! `crate::rules::missing_required_field`): a task/subtask whose property
//! attaches a `[Form.X]` is flagged when its body is missing one of that
//! form's `__Label:__ value` field lines entirely.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn pending_branch_requires_only_base_form_answers() {
    let dir = tempdir().unwrap();
    let config = "\
[Field.base]
label = \"Base\"
[Field.approval]
label = \"Approval\"
[Field.rejection]
label = \"Rejection\"
[Form.base]
fields = [\"base\"]
[Form.approval]
fields = [\"approval\"]
[Form.rejection]
fields = [\"rejection\"]
[Properties.review]
form = \"base\"
[Properties.review.passed]
form = \"approval\"
[Properties.review.failed]
form = \"rejection\"
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] #review:...
  __Base:__ base answer
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn selected_branch_missing_its_form_answer_reports_e024_and_preserves_previous_answers() {
    let dir = tempdir().unwrap();
    let config = "\
[Field.base]
label = \"Base\"
[Field.approval]
label = \"Approval\"
[Field.rejection]
label = \"Rejection\"
[Form.base]
fields = [\"base\"]
[Form.approval]
fields = [\"approval\"]
[Form.rejection]
fields = [\"rejection\"]
[Properties.review]
form = \"base\"
[Properties.review.passed]
form = \"approval\"
[Properties.review.failed]
form = \"rejection\"
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] #review:failed
  __Base:__ base answer
  __Approval:__ previous approval
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout.matches("error[").count(), 1, "{stdout}");
    assert!(stdout.contains("E024"), "{stdout}");
    assert!(stdout.contains("Rejection"), "{stdout}");
    assert_eq!(
        fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap(),
        file_content
    );
}

#[test]
fn selected_branch_accepts_filled_forms_with_previous_branch_answers_present() {
    let dir = tempdir().unwrap();
    let config = "\
[Field.base]
label = \"Base\"
[Field.approval]
label = \"Approval\"
[Field.rejection]
label = \"Rejection\"
[Form.base]
fields = [\"base\"]
[Form.approval]
fields = [\"approval\"]
[Form.rejection]
fields = [\"rejection\"]
[Properties.review]
form = \"base\"
[Properties.review.passed]
form = \"approval\"
[Properties.review.failed]
form = \"rejection\"
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] #review:failed
  __Base:__ base answer
  __Approval:__ previous approval
  __Rejection:__ rejection answer
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

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

/// Two distinct properties (`onboarding`/`personal_info` as above, plus
/// `equipment`/`equipment_request`), each attaching its own, entirely
/// separate form -- both can be declared on the same task at once. No
/// label/field-name collisions between the two forms are assumed here.
fn write_two_property_config(dir: &std::path::Path) {
    let config = "\
[Field.first_name]
label = \"First Name\"
[Field.email]
label = \"Email\"
[Form.personal_info]
fields = [\"first_name\", \"email\"]
[Properties.onboarding]
form = \"personal_info\"
[Field.device]
label = \"Device\"
[Field.os]
label = \"OS\"
[Form.equipment_request]
fields = [\"device\", \"os\"]
[Properties.equipment]
form = \"equipment_request\"
";
    fs::write(dir.join("mdagile.toml"), config).unwrap();
}

#[test]
fn two_properties_with_two_forms_on_the_same_task_are_each_flagged_independently() {
    let dir = tempdir().unwrap();
    write_two_property_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding #equipment
  __First Name:__ John
  __Device:__ laptop
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E024"), "stdout: {stdout:?}");
    // Missing from `personal_info` and from `equipment_request` respectively.
    assert!(stdout.contains("Email"), "stdout: {stdout:?}");
    assert!(stdout.contains("OS"), "stdout: {stdout:?}");
}

#[test]
fn two_properties_with_two_forms_only_the_unsatisfied_one_is_flagged() {
    let dir = tempdir().unwrap();
    write_two_property_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding #equipment
  __First Name:__ John
  __Email:__ john@example.com
  __Device:__ laptop
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E024"), "stdout: {stdout:?}");
    assert!(stdout.contains("OS"), "stdout: {stdout:?}");
    // `personal_info`'s fields are all present -- its labels must not show
    // up as missing.
    assert!(!stdout.contains("Missing required field: \"Email\""));
    assert!(!stdout.contains("Missing required field: \"First Name\""));
}

#[test]
fn two_properties_with_two_forms_both_fully_satisfied_is_clean() {
    let dir = tempdir().unwrap();
    write_two_property_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #onboarding #equipment
  __First Name:__ John
  __Email:__ john@example.com
  __Device:__ laptop
  __OS:__ Linux
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
