//! Acceptance tests for jumping to tasks via `textDocument/declaration`.
//!
//! Note on LSP method choice: we are intentionally repurposing/abusing the standard
//! `textDocument/declaration` (commonly bound to `gD` / `Shift+F12` in editors) to jump
//! to the highest-priority open task. This makes the jumping feature immediately
//! available out-of-the-box without requiring users to configure custom keybindings or
//! client extensions.

use super::helpers::start_project_session;

#[test]
fn lsp_goto_declaration_jumps_to_highest_priority_open_task() {
    // Arrange
    let (mut session, file_uri) = start_project_session("");
    let file_content = "\
- [x] done task
- [ ] highest priority open task
- [ ] another open task
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.goto_declaration(&file_uri, 2, 0, 0);

    // Assert
    assert!(
        !response["result"].is_null(),
        "expected a location result, got: {response}"
    );
    assert_eq!(
        response["result"]["uri"].as_str().unwrap(),
        file_uri,
        "GoTo Declaration should point to the tasks file"
    );
    assert_eq!(
        response["result"]["range"]["start"]["line"], 1,
        "GoTo Declaration should point to line 1 (highest priority open task)"
    );
}

#[test]
fn lsp_goto_declaration_returns_null_when_all_tasks_done() {
    // Arrange
    let (mut session, file_uri) = start_project_session("");
    let file_content = "\
- [x] done task
- [-] cancelled task
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.goto_declaration(&file_uri, 2, 0, 0);

    // Assert
    assert!(
        response["result"].is_null(),
        "expected null when no open task exists, got: {response}"
    );
}

#[test]
fn lsp_goto_declaration_jumps_across_files_to_highest_priority_task() {
    // Arrange
    let project_root = tempfile::tempdir().unwrap();
    let root_uri = super::helpers::file_uri(project_root.path());

    let file1_content = "\
- [x] done task in file 1
";
    let file2_content = "\
- [ ] active task in file 2
";
    let file1_path = project_root.path().join("01_done.agile.md");
    let file2_path = project_root.path().join("02_todo.agile.md");
    std::fs::write(&file1_path, file1_content).unwrap();
    std::fs::write(&file2_path, file2_content).unwrap();

    let file1_uri = super::helpers::file_uri(&file1_path);
    let file2_uri = super::helpers::file_uri(&file2_path);

    std::mem::forget(project_root);
    let mut session = super::helpers::LspSession::start_with_root_uri(Some(&root_uri));

    session.open_document(&file1_uri, file1_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act - cursor is in file1, where all tasks are done
    let response = session.goto_declaration(&file1_uri, 2, 0, 0);

    // Assert - should jump to file2's active task
    assert!(
        !response["result"].is_null(),
        "expected a location result, got: {response}"
    );
    assert_eq!(
        response["result"]["uri"].as_str().unwrap(),
        file2_uri,
        "GoTo Declaration should point to 02_todo.agile.md"
    );
    assert_eq!(
        response["result"]["range"]["start"]["line"], 0,
        "GoTo Declaration should point to line 0 of 02_todo.agile.md"
    );
}

#[test]
fn lsp_goto_declaration_uses_unsaved_current_file_in_workspace_order() {
    // Arrange
    let project_root = tempfile::tempdir().unwrap();
    let first_path = project_root.path().join("01_tasks.agile.md");
    let second_path = project_root.path().join("02_tasks.agile.md");
    let disk_content = "\
- [x] first task completed on disk
";
    let live_content = "\
- [ ] first task reopened in editor
";
    let second_content = "\
- [ ] later task
";
    std::fs::write(&first_path, disk_content).unwrap();
    std::fs::write(&second_path, second_content).unwrap();
    let root_uri = super::helpers::file_uri(project_root.path());
    let file_uri = super::helpers::file_uri(&first_path);
    let mut session = super::helpers::LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(&file_uri, live_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.goto_declaration(&file_uri, 2, 0, 0);

    // Assert
    assert_eq!(response["result"]["uri"], file_uri);
    assert_eq!(response["result"]["range"]["start"]["line"], 0);
}

#[test]
fn lsp_goto_declaration_skips_task_closed_in_unsaved_current_file() {
    // Arrange
    let project_root = tempfile::tempdir().unwrap();
    let first_path = project_root.path().join("01_tasks.agile.md");
    let second_path = project_root.path().join("02_tasks.agile.md");
    let disk_content = "\
- [ ] first task open on disk
";
    let live_content = "\
- [x] first task completed in editor
";
    let second_content = "\
- [ ] later task
";
    std::fs::write(&first_path, disk_content).unwrap();
    std::fs::write(&second_path, second_content).unwrap();
    let root_uri = super::helpers::file_uri(project_root.path());
    let file_uri = super::helpers::file_uri(&first_path);
    let second_uri = super::helpers::file_uri(&second_path);
    let mut session = super::helpers::LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(&file_uri, live_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.goto_declaration(&file_uri, 2, 0, 0);

    // Assert
    assert_eq!(response["result"]["uri"], second_uri);
    assert_eq!(response["result"]["range"]["start"]["line"], 0);
}
