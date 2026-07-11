//! Acceptance tests for `agile check` group-member config validation.
//!
//! A `[Groups.X]` entry's `members` list must only reference names that
//! have a corresponding `[Users.X]` entry; an unknown member is a fatal
//! config error (exit 1), consistent with other config-level errors.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_group_member_referencing_undefined_user() {
    let dir = tempdir().unwrap();
    let file_content = "\
[Groups.devs]
members = [\"ghost\"]
";
    fs::write(dir.path().join("mdagile.toml"), file_content).unwrap();
    let file_content = "\
- [ ] top
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.to_lowercase().contains("invalid config"),
        "stderr: {stderr:?}"
    );
    assert!(stderr.contains("ghost"), "stderr: {stderr:?}");
}

#[test]
fn allows_group_with_all_members_defined() {
    let dir = tempdir().unwrap();
    let file_content = "\
[Users.alice]

[Groups.devs]
members = [\"alice\"]
";
    fs::write(dir.path().join("mdagile.toml"), file_content).unwrap();
    let file_content = "\
- [ ] top
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
