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
fn flags_subtask_with_wrong_indentation() {
    let dir = tempdir().unwrap();
    // 3 spaces instead of 2 for a depth-1 subtask
    let content = "\
- [ ] top task
   - [ ] subtask with 3 spaces
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E002"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_correctly_indented_subtask() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] top task
  - [ ] depth 1
    - [ ] depth 2
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
