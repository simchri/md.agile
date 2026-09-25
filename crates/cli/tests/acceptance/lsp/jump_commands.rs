//! Acceptance tests for the `mdagile.jump.*` custom `workspace/executeCommand`
//! commands: `highestPriorityOpen`, `highestPriorityMy`, `nextOpen`,
//! `previousOpen`, `nextMy`, `previousMy`.
//!
//! Each command takes `[currentUri]` (highestPriority* commands) or
//! `[currentUri, currentLine]` (next*/previous* commands) as its
//! `arguments`, and jumps by asking the client to navigate via
//! `window/showDocument` rather than returning a `Location` directly (there's
//! no other client-visible way for a custom command to move the cursor).

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
fn lsp_jump_highest_priority_open_command_shows_document_at_highest_priority_task() {
    // Arrange
    let (mut session, file_uri) = super::helpers::start_project_session("");
    let file_content = "\
- [x] done task
- [ ] open task
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let (show_document, response) = session.execute_command(
        2,
        "mdagile.jump.highestPriorityOpen",
        vec![serde_json::json!(file_uri)],
    );

    // Assert
    assert_eq!(show_document["params"]["uri"].as_str().unwrap(), file_uri);
    assert_eq!(
        show_document["params"]["selection"]["start"]["line"], 1,
        "should show the document at line 1 (the open task)"
    );
    assert_eq!(response["result"], serde_json::json!(true));
}

#[test]
fn lsp_jump_highest_priority_my_command_shows_document_at_my_highest_priority_task() {
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
    let (show_document, response) = session.execute_command(
        2,
        "mdagile.jump.highestPriorityMy",
        vec![serde_json::json!(file_uri)],
    );

    // Assert
    assert_eq!(
        show_document["params"]["selection"]["start"]["line"], 1,
        "should show the document at line 1 (alice's task)"
    );
    assert_eq!(response["result"], serde_json::json!(true));
}

#[test]
fn lsp_jump_next_open_command_shows_document_at_next_open_task_after_cursor() {
    // Arrange
    let (mut session, file_uri) = super::helpers::start_project_session("");
    let file_content = "\
- [ ] first open task
- [x] done task
- [ ] second open task
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act: cursor on line 0 (first open task) -> next open task is line 2.
    let (show_document, response) = session.execute_command(
        2,
        "mdagile.jump.nextOpen",
        vec![serde_json::json!(file_uri), serde_json::json!(0)],
    );

    // Assert
    assert_eq!(show_document["params"]["selection"]["start"]["line"], 2);
    assert_eq!(response["result"], serde_json::json!(true));
}

#[test]
fn lsp_jump_next_open_command_returns_false_when_no_task_follows_cursor() {
    // Arrange
    let (mut session, file_uri) = super::helpers::start_project_session("");
    let file_content = "\
- [ ] only open task
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act: cursor already past the only open task.
    let (show_document, response) = session.execute_command(
        2,
        "mdagile.jump.nextOpen",
        vec![serde_json::json!(file_uri), serde_json::json!(0)],
    );

    // Assert
    assert!(
        show_document.is_null(),
        "no window/showDocument should be sent when there's no next task"
    );
    assert_eq!(response["result"], serde_json::json!(false));
}

#[test]
fn lsp_jump_previous_open_command_shows_document_at_previous_open_task_before_cursor() {
    // Arrange
    let (mut session, file_uri) = super::helpers::start_project_session("");
    let file_content = "\
- [ ] first open task
- [x] done task
- [ ] second open task
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act: cursor on line 2 (second open task) -> previous open task is line 0.
    let (show_document, response) = session.execute_command(
        2,
        "mdagile.jump.previousOpen",
        vec![serde_json::json!(file_uri), serde_json::json!(2)],
    );

    // Assert
    assert_eq!(show_document["params"]["selection"]["start"]["line"], 0);
    assert_eq!(response["result"], serde_json::json!(true));
}

#[test]
fn lsp_jump_next_my_command_skips_tasks_not_eligible_to_me() {
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
- [ ] task for alice @alice
- [ ] task for bob @bob
- [ ] another task for alice @alice
";
    let task_path = project_root.path().join("tasks.agile.md");
    fs::write(&task_path, file_content).unwrap();

    let root_uri = super::helpers::file_uri(project_root.path());
    let file_uri = super::helpers::file_uri(&task_path);

    std::mem::forget(project_root);
    let mut session = super::helpers::LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act: cursor on line 0 (alice's first task) -> skip bob's task on line 1,
    // land on alice's second task on line 2.
    let (show_document, response) = session.execute_command(
        2,
        "mdagile.jump.nextMy",
        vec![serde_json::json!(file_uri), serde_json::json!(0)],
    );

    // Assert
    assert_eq!(show_document["params"]["selection"]["start"]["line"], 2);
    assert_eq!(response["result"], serde_json::json!(true));
}

#[test]
fn lsp_jump_previous_my_command_skips_tasks_not_eligible_to_me() {
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
- [ ] task for alice @alice
- [ ] task for bob @bob
- [ ] another task for alice @alice
";
    let task_path = project_root.path().join("tasks.agile.md");
    fs::write(&task_path, file_content).unwrap();

    let root_uri = super::helpers::file_uri(project_root.path());
    let file_uri = super::helpers::file_uri(&task_path);

    std::mem::forget(project_root);
    let mut session = super::helpers::LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act: cursor on line 2 (alice's second task) -> skip bob's task on line
    // 1, land on alice's first task on line 0.
    let (show_document, response) = session.execute_command(
        2,
        "mdagile.jump.previousMy",
        vec![serde_json::json!(file_uri), serde_json::json!(2)],
    );

    // Assert
    assert_eq!(show_document["params"]["selection"]["start"]["line"], 0);
    assert_eq!(response["result"], serde_json::json!(true));
}
