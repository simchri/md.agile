//! Acceptance tests for `agile check` config validation.
//!
//! Unknown keys in `mdagile.toml` (unrecognized top-level sections, or
//! unrecognized fields inside `[Properties.X]`/`[Users.X]`/`[Groups.X]`)
//! are a fatal config error: the CLI exits 1 with a message on stderr,
//! consistent with other config-level errors (conflicting config files,
//! mismatched `subtasks_allow_cancel` length).

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_unknown_top_level_section() {
    let dir = tempdir().unwrap();
    let file_content = "\
[Typo]
foo = 1
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
}

#[test]
fn flags_unknown_key_in_property_section() {
    let dir = tempdir().unwrap();
    let file_content = "\
[Properties.feature]
subtsaks = [\"dev implementation\"]
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
}

#[test]
fn known_config_keys_still_pass() {
    let dir = tempdir().unwrap();
    let file_content = "\
[Properties.feature]
subtasks = [\"dev implementation\"]

[Users.alice]
emails = [\"alice@example.com\"]

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
