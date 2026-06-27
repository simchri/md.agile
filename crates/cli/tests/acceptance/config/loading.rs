use crate::helpers::{run_check, run_list};
use std::fs;
use tempfile::tempdir;

#[test]
fn conflicting_config_files_exit_nonzero_with_error_message() {
    let dir = tempdir().unwrap();
    let config_a = "\
[Properties.feature]
";
    fs::write(dir.path().join("mdagile.toml"), config_a).unwrap();
    let config_b = "\
[Properties.bug]
";
    fs::write(dir.path().join(".mdagile.toml"), config_b).unwrap();

    let out = run_check(dir.path());

    assert_ne!(out.status.code(), Some(0), "expected non-zero exit");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("conflicting config"), "stderr: {stderr:?}");
}

#[test]
fn invalid_toml_config_exits_nonzero_with_error_message() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();

    let out = run_check(dir.path());

    assert_ne!(out.status.code(), Some(0), "expected non-zero exit");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("invalid config") || stderr.contains("config"),
        "stderr: {stderr:?}"
    );
}

#[test]
fn valid_config_does_not_prevent_normal_operation() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn config_error_is_reported_regardless_of_subcommand() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();

    let out = run_list(dir.path());

    assert_ne!(out.status.code(), Some(0), "expected non-zero exit");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(!stderr.is_empty(), "expected error on stderr");
}
