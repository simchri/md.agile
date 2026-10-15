use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_task_marked_done_with_an_unresolved_branch_property() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
subtasks = [\"document review findings\"]
[Properties.review.passed]
neighbortasks = [\"publish feature\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    // See README.vision.md "Branch Properties": this is the exact invalid
    // example given there.
    let content = "\
- [x] perform #review...
  - [x] \"document review findings\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E021"), "stdout: {stdout:?}");
    assert!(stdout.contains("#review..."), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_a_still_open_task_with_an_unresolved_branch_property() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
subtasks = [\"document review findings\"]
[Properties.review.passed]
neighbortasks = [\"publish feature\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] perform #review...
  - [ ] \"document review findings\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E021"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_a_done_task_whose_branch_property_is_resolved() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
subtasks = [\"document review findings\"]
[Properties.review.passed]
neighbortasks = [\"publish feature\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] build something
  - [x] perform #review:passed
    - [x] \"document review findings\"
  - [ ] \"publish feature\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E021"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_a_cancelled_task_with_an_unresolved_branch_property() {
    // Cancelling abandons the task without needing to resolve its outcome —
    // only marking it Done (E021's actual concern) requires resolution.
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [-] perform #review...
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E021"), "stdout: {stdout:?}");
}

#[test]
fn flags_a_done_task_with_two_distinct_unresolved_branch_properties_naming_both() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
[Properties.approval]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [x] perform #review... and #approval...
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E021"), "stdout: {stdout:?}");
    assert!(stdout.contains("#review..."), "stdout: {stdout:?}");
    assert!(stdout.contains("#approval..."), "stdout: {stdout:?}");
}
