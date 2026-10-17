//! Acceptance tests for E027 "Colliding Field Labels" (see
//! `crate::rules::colliding_field_labels`): a task/subtask on which two
//! distinct properties each attach a `[Form.X]` is flagged when those two
//! forms declare a field with the same display label.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

fn write_colliding_config(dir: &std::path::Path) {
    let config = "\
[Field.name_a]
label = \"Name\"
[Form.form_a]
fields = [\"name_a\"]
[Properties.propertyA]
form = \"form_a\"

[Field.name_b]
label = \"Name\"
[Form.form_b]
fields = [\"name_b\"]
[Properties.propertyB]
form = \"form_b\"
";
    fs::write(dir.join("mdagile.toml"), config).unwrap();
}

#[test]
fn flags_two_properties_whose_forms_declare_the_same_field_label() {
    let dir = tempdir().unwrap();
    write_colliding_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #propertyA #propertyB
  __Name:__ John
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E027"), "stdout: {stdout:?}");
    assert!(stdout.contains("Name"), "stdout: {stdout:?}");
    assert!(stdout.contains("propertyA"), "stdout: {stdout:?}");
    assert!(stdout.contains("propertyB"), "stdout: {stdout:?}");
}

#[test]
fn a_single_property_with_a_form_is_never_flagged() {
    let dir = tempdir().unwrap();
    write_colliding_config(dir.path());
    let file_content = "\
- [ ] new colleague: John #propertyA
  __Name:__ John
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
fn two_properties_with_forms_having_distinct_labels_are_not_flagged() {
    let dir = tempdir().unwrap();
    let config = "\
[Field.first_name]
label = \"First Name\"
[Form.personal_info]
fields = [\"first_name\"]
[Properties.onboarding]
form = \"personal_info\"

[Field.device]
label = \"Device\"
[Form.equipment_request]
fields = [\"device\"]
[Properties.equipment]
form = \"equipment_request\"
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] new colleague: John #onboarding #equipment
  __First Name:__ John
  __Device:__ laptop
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
fn two_properties_sharing_the_same_form_are_not_flagged() {
    let dir = tempdir().unwrap();
    let config = "\
[Field.name]
label = \"Name\"
[Form.shared_form]
fields = [\"name\"]
[Properties.propertyA]
form = \"shared_form\"
[Properties.propertyB]
form = \"shared_form\"
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] new colleague: John #propertyA #propertyB
  __Name:__ John
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
