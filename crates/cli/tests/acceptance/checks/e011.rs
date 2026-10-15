use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_quoted_subtask_not_required_by_any_property() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature]
subtasks = [\"PO review\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    // "sneaky" is quoted but not required by #feature.
    let content = "\
- [ ] #feature add login
  - [ ] \"sneaky\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E011"), "stdout: {stdout:?}");
    assert!(stdout.contains("sneaky"), "stdout: {stdout:?}");
}

#[test]
fn flags_quoted_subtask_on_task_with_no_property() {
    let dir = tempdir().unwrap();
    let file_content = "";
    fs::write(dir.path().join("mdagile.toml"), file_content).unwrap();
    let content = "\
- [ ] plain task
  - [ ] \"quoted without reason\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E011"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_required_quoted_subtask() {
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
fn does_not_flag_a_quoted_subtask_required_only_by_a_resolved_branch() {
    // Regression test for a bug found via sample_2.agile.md: "review passed
    // subtask" is required by `[Properties.review.passed]`, not by the base
    // `[Properties.review]` -- but once the property is resolved to that
    // branch (`#review:passed`), the branch's own `subtasks` apply exactly
    // like the base property's (see `missing_required_subtasks`, which
    // already merges them in for E010/E012). E011 must honor that same
    // merge, not just the base property's own list, or it incorrectly
    // flags a perfectly valid required quoted subtask.
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
subtasks = [\"document review findings\"]
[Properties.review.passed]
neighbortasks = [\"publish feature\"]
subtasks = [\"review passed subtask\"]
[Properties.review.failed]
neighbortasks = [\"create follow up task for fixes\"]
subtasks = [\"review failed subtask\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] a task
  - [ ] some sub task that needs #review:passed
    - [ ] \"document review findings\"
    - [ ] \"review passed subtask\"
  - [ ] \"publish feature\"
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
