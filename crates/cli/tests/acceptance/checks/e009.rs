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
fn flags_undefined_assignment() {
    let dir = tempdir().unwrap();
    // Config defines no users; @alice is therefore undefined
    fs::write(dir.path().join("mdagile.toml"), "[Properties.feat]\n").unwrap();
    fs::write(dir.path().join("a.agile.md"), "- [ ] @alice do the thing\n").unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E009"), "stdout: {stdout:?}");
    assert!(stdout.contains("alice"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_declared_assignment() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("mdagile.toml"), "[Users.alice]\n").unwrap();
    fs::write(dir.path().join("a.agile.md"), "- [ ] @alice do the thing\n").unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
