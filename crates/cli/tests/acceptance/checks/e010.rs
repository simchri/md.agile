use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_task_missing_required_subtask() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature]
subtasks = [\"PO review\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    // #feature task has no "PO review" subtask
    let content = "\
- [ ] #feature add login
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E010"), "stdout: {stdout:?}");
    assert!(stdout.contains("PO review"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_task_with_required_subtask_present() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature]
subtasks = [\"PO review\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] #feature add login
  - [ ] \"PO review\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

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
