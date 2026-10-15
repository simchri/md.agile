//! Edge cases for Branch Properties beyond the core E021/E022 happy paths
//! already covered in `checks/e021.rs`/`checks/e022.rs`: overlapping
//! requirements between a resolved branch and other properties, and the
//! interaction with cancellation.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn a_resolved_branchs_required_subtask_identical_to_the_base_propertys_own_is_satisfied_once() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
subtasks = [\"sign off\"]
[Properties.review.passed]
subtasks = [\"sign off\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] build something
  - [ ] perform #review:passed
    - [ ] \"sign off\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E010"), "stdout: {stdout:?}");
}

#[test]
fn a_resolved_branchs_required_subtask_identical_to_a_different_propertys_own_is_satisfied_once() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature]
subtasks = [\"test\"]
[Properties.review]
subtasks = [\"document review findings\"]
[Properties.review.passed]
subtasks = [\"test\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] #feature and #review:passed task
  - [ ] \"document review findings\"
  - [ ] \"test\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E010"), "stdout: {stdout:?}");
}

#[test]
fn a_resolved_branchs_required_neighbor_identical_to_the_base_propertys_own_is_satisfied_once() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
neighbortasks = [\"sign off meeting\"]
[Properties.review.passed]
neighbortasks = [\"sign off meeting\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] build something
  - [ ] perform #review:passed
  - [ ] \"sign off meeting\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E019"), "stdout: {stdout:?}");
}

#[test]
fn branchs_allow_cancel_merges_with_the_base_propertys_for_an_identical_title() {
    // The base property disallows cancelling "sign off", but the resolved
    // branch's own copy of that same required title allows it. The more
    // permissive setting wins, so cancelling it must not trip E012.
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
subtasks = [\"sign off\"]
subtasks_allow_cancel = [false]
[Properties.review.passed]
subtasks = [\"sign off\"]
subtasks_allow_cancel = [true]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] build something
  - [ ] perform #review:passed
    - [-] \"sign off\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E012"), "stdout: {stdout:?}");
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
