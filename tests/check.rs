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
fn clean_project_exits_zero_with_no_output() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] top
  - [ ] proper sub
- [x] done
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(out.status.success(), "expected exit 0, got {:?}", out.status);
    assert!(out.stdout.is_empty(), "stdout: {}", String::from_utf8_lossy(&out.stdout));
    assert!(out.stderr.is_empty(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn empty_project_exits_zero() {
    // No *.agile.md files at all is also "clean".
    let dir = tempdir().unwrap();
    let out = run_check(dir.path());
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}

#[test]
fn flags_orphaned_indented_top_level_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] proper top

  - [ ] orphan indented
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Single issue in ESLint-style format: error header, location line, source line, pointer, help.
    // Should contain the location and error details.
    assert!(stdout.contains("a.agile.md:3:"), "stdout: {stdout:?}");
    assert!(stdout.contains("orphaned indented task"), "stdout: {stdout:?}");
    assert!(stdout.contains("error[E001]"), "stdout: {stdout:?}");
}

#[test]
fn aggregates_issues_across_multiple_files() {
    let dir = tempdir().unwrap();
    let file_a = "\
- [ ] top

  - [ ] orphan a
";
    let file_b = "\
- [ ] top

    - [ ] orphan b
";
    fs::write(dir.path().join("a.agile.md"), file_a).unwrap();
    fs::write(dir.path().join("b.agile.md"), file_b).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Two issues in ESLint-style format, each containing error details.
    // Check that both files' errors are present.
    assert!(stdout.contains("a.agile.md:3:"), "stdout: {stdout:?}");
    assert!(stdout.contains("b.agile.md:3:"), "stdout: {stdout:?}");
    assert!(stdout.contains("error[E001]"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_proper_subtask_under_a_real_parent() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent
  - [ ] real subtask
    - [ ] grandchild
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());
    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout),
    );
    assert!(out.stdout.is_empty());
}
