use crate::helpers::run_agile_clean_env;
use std::fs;
use tempfile::tempdir;

#[test]
fn default_no_editor_set_exits_nonzero() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] some task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // No VISUAL or EDITOR in environment
    let out = run_agile_clean_env(dir.path(), &[], &[]);

    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.to_lowercase().contains("editor"),
        "expected editor-related error: {stderr:?}"
    );
}

#[test]
fn default_no_active_tasks_exits_zero() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] all done
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // Even without an editor set, no tasks → normal exit before trying to open editor
    let out = run_agile_clean_env(dir.path(), &[], &[]);

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn default_empty_project_exits_zero() {
    let dir = tempdir().unwrap();

    let out = run_agile_clean_env(dir.path(), &[], &[]);

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn default_with_editor_set_to_echo_succeeds() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] a task to open
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // `echo` succeeds and ignores its arguments: good stand-in for a real editor
    let out = run_agile_clean_env(dir.path(), &[], &[("EDITOR", "echo")]);

    assert!(
        out.status.success(),
        "expected success; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
