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
fn flags_missing_space_after_box() {
    let dir = tempdir().unwrap();
    // No space between `]` and task title
    fs::write(dir.path().join("a.agile.md"), "- [ ]task title\n").unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E005"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_task_with_space_after_box() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.agile.md"), "- [ ] task title\n").unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
