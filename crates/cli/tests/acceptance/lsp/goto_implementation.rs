//! Acceptance tests for jumping to tasks via `textDocument/implementation`.
//!
//! Note on LSP method choice: we are intentionally repurposing/abusing the standard
//! `textDocument/implementation` (commonly bound to `gi` / `Ctrl+F12` in editors) to jump
//! to the highest-priority open task eligible to the current user (my highest priority task).
//! This makes the jumping feature immediately available out-of-the-box without requiring
//! users to configure custom keybindings or client extensions.

use std::fs;
use std::process::Command;

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git command failed to start");
    assert!(status.success(), "git {args:?} failed");
}

#[test]
fn lsp_goto_implementation_jumps_to_highest_priority_my_task() {
    // Arrange
    let project_root = tempfile::tempdir().unwrap();
    git(project_root.path(), &["init", "-q"]);
    git(
        project_root.path(),
        &["config", "user.email", "alice@example.com"],
    );
    git(project_root.path(), &["config", "user.name", "Alice"]);

    let config_toml = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(project_root.path().join("mdagile.toml"), config_toml).unwrap();

    let file_content = "\
- [ ] task for bob @bob
- [ ] task for alice @alice
- [ ] unassigned task
";
    let task_path = project_root.path().join("tasks.agile.md");
    fs::write(&task_path, file_content).unwrap();

    let root_uri = super::helpers::file_uri(project_root.path());
    let file_uri = super::helpers::file_uri(&task_path);

    std::mem::forget(project_root);
    let mut session = super::helpers::LspSession::start_with_root_uri(Some(&root_uri));

    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.goto_implementation(&file_uri, 2, 0, 0);

    // Assert
    assert!(
        !response["result"].is_null(),
        "expected a location result, got: {response}"
    );
    assert_eq!(
        response["result"]["uri"].as_str().unwrap(),
        file_uri,
        "GoTo Implementation should point to tasks.agile.md"
    );
    assert_eq!(
        response["result"]["range"]["start"]["line"], 1,
        "GoTo Implementation should point to line 1 (task for alice)"
    );
}

#[test]
fn lsp_goto_implementation_returns_null_when_no_my_tasks_eligible() {
    // Arrange
    let project_root = tempfile::tempdir().unwrap();
    git(project_root.path(), &["init", "-q"]);
    git(
        project_root.path(),
        &["config", "user.email", "alice@example.com"],
    );
    git(project_root.path(), &["config", "user.name", "Alice"]);

    let config_toml = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(project_root.path().join("mdagile.toml"), config_toml).unwrap();

    let file_content = "\
- [ ] task for bob @bob
";
    let task_path = project_root.path().join("tasks.agile.md");
    fs::write(&task_path, file_content).unwrap();

    let root_uri = super::helpers::file_uri(project_root.path());
    let file_uri = super::helpers::file_uri(&task_path);

    std::mem::forget(project_root);
    let mut session = super::helpers::LspSession::start_with_root_uri(Some(&root_uri));

    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.goto_implementation(&file_uri, 2, 0, 0);

    // Assert
    assert!(
        response["result"].is_null(),
        "expected null when no eligible task exists, got: {response}"
    );
}
