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
fn flags_body_line_with_wrong_indentation() {
    let dir = tempdir().unwrap();
    // Body lines for a top-level task should be indented 2 spaces; 3 is wrong.
    let content = "\
- [ ] task title
  correct body line
   wrong body line
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E003"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_correct_body_indentation() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] task title
  first body line
  second body line
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
