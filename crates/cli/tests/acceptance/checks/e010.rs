use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::tempdir;

fn run_check(cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agile"))
        .arg("check")
        .current_dir(cwd)
        .output()
        .expect("failed to spawn `agile check`")
}

#[test]
fn flags_task_missing_required_subtask() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("mdagile.toml"),
        "[Properties.feature]\nsubtasks = [\"PO review\"]\n",
    )
    .unwrap();
    // #feature task has no "PO review" subtask
    fs::write(dir.path().join("a.agile.md"), "- [ ] #feature add login\n").unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E010"), "stdout: {stdout:?}");
    assert!(stdout.contains("PO review"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_task_with_required_subtask_present() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("mdagile.toml"),
        "[Properties.feature]\nsubtasks = [\"PO review\"]\n",
    )
    .unwrap();
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
