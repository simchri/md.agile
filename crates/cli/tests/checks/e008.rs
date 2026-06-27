//! Acceptance tests for `agile check`.
//!
//! Each test spawns the real `mdagile` binary in a tempdir and asserts on its
//! exit code, stdout, and stderr — exercising CLI parsing, file walking,
//! issue formatting, and exit-code behavior end-to-end.

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
fn e008_undefined_property_marker_is_flagged() {
    let dir = tempdir().unwrap();
    // Config with one defined property
    fs::write(dir.path().join("mdagile.toml"), "[Properties.feature]\n").unwrap();
    // Task uses #bug which is NOT defined
    fs::write(dir.path().join("a.agile.md"), "- [ ] #bug fix the thing\n").unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E008"), "stdout: {stdout:?}");
    assert!(stdout.contains("bug"), "stdout: {stdout:?}");
}

#[test]
fn e008_defined_property_marker_is_not_flagged() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("mdagile.toml"), "[Properties.feature]\n").unwrap();
    fs::write(
        dir.path().join("a.agile.md"),
        "- [ ] #feature add new thing\n",
    )
    .unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn e008_undefined_property_without_config_file_is_also_flagged() {
    let dir = tempdir().unwrap();
    // No mdagile.toml at all — any #property usage is an error
    fs::write(
        dir.path().join("a.agile.md"),
        "- [ ] #feature add new thing\n",
    )
    .unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E008"), "stdout: {stdout:?}");
}

