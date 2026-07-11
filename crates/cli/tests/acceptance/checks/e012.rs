use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_cancelled_required_subtask_when_not_allowed() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature]
subtasks = [\"PO review\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] #feature add login
  - [-] \"PO review\"
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E012"), "stdout: {stdout:?}");
    assert!(stdout.contains("PO review"), "stdout: {stdout:?}");
}

#[test]
fn allows_cancelled_required_subtask_when_configured() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature]
subtasks = [\"PO review\"]
subtasks_allow_cancel = [true]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] #feature add login
  - [-] \"PO review\"
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
