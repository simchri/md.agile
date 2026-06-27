//! Acceptance tests for `agile check`.
//!
//! Each test spawns the real `mdagile` binary in a tempdir and asserts on its
//! exit code, stdout, and stderr — exercising CLI parsing, file walking,
//! issue formatting, and exit-code behavior end-to-end.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn undefined_property_marker_is_flagged() {
    let dir = tempdir().unwrap();
    // Config with one defined property
    let config = "\
[Properties.feature]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    // Task uses #bug which is NOT defined
    let content = "\
- [ ] #bug fix the thing
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E008"), "stdout: {stdout:?}");
    assert!(stdout.contains("bug"), "stdout: {stdout:?}");
}

#[test]
fn defined_property_marker_is_not_flagged() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] #feature add new thing
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
fn undefined_property_without_config_file_is_also_flagged() {
    let dir = tempdir().unwrap();
    // No mdagile.toml at all — any #property usage is an error
    let content = "\
- [ ] #feature add new thing
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E008"), "stdout: {stdout:?}");
}
