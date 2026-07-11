//! Acceptance tests for `agile check`.
//!
//! Each test spawns the real `mdagile` binary in a tempdir and asserts on its
//! exit code, stdout, and stderr — exercising CLI parsing, file walking,
//! issue formatting, and exit-code behavior end-to-end.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn clean_project_exits_zero_with_no_output() {
    let dir = tempdir().unwrap();
    // Not a git repo: silence the (unrelated) E013 "not a git repo" warning
    // so this test can assert on rule-cleanliness output alone.
    let config = "\
[General]
warn_when_not_a_git_repo = false
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
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
