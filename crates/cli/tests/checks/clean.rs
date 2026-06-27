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

    assert!(
        out.status.success(),
        "expected exit 0, got {:?}",
        out.status
    );
    assert!(
        out.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        out.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn empty_project_exits_zero() {
    // No *.agile.md files at all is also "clean".
    let dir = tempdir().unwrap();
    let out = run_check(dir.path());
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}
