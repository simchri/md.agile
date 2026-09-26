//! Acceptance tests demonstrating that jump/navigation actions
//! (`textDocument/declaration`, `textDocument/implementation`, and the
//! `mdagile.jump.*` commands) consider subtasks, not just top-level tasks:
//! a jump lands on the actual actionable (sub)task — mirroring
//! `rules::next_task::is_next_task`/`find_next_actionable`, the same logic
//! `agile task next`/`done` already use — rather than always stopping at a
//! top-level task's own line even when it still has open children.

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
fn lsp_goto_declaration_jumps_into_open_subtask_not_parent_line() {
    // Arrange
    let (mut session, file_uri) = super::helpers::start_project_session("");
    let file_content = "\
- [x] done task
- [ ] parent with subtasks
  - [ ] first subtask
  - [ ] second subtask
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.goto_declaration(&file_uri, 2, 0, 0);

    // Assert: the parent (line 1) still has open descendant work, so the
    // actionable task is its first open subtask (line 2), not the parent
    // itself.
    assert_eq!(
        response["result"]["range"]["start"]["line"], 2,
        "GoTo Declaration should point to line 2 (first open subtask), got: {response}"
    );
}

#[test]
fn lsp_goto_declaration_jumps_to_parent_once_every_subtask_is_resolved() {
    // Arrange
    let (mut session, file_uri) = super::helpers::start_project_session("");
    let file_content = "\
- [ ] parent with resolved subtasks
  - [x] first subtask
  - [-] second subtask
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.goto_declaration(&file_uri, 2, 0, 0);

    // Assert: every descendant is done/cancelled, so there's nothing left
    // to delegate to — the parent itself (line 0) is the actionable unit.
    assert_eq!(
        response["result"]["range"]["start"]["line"], 0,
        "GoTo Declaration should point to line 0 (the parent itself), got: {response}"
    );
}

#[test]
fn lsp_jump_next_open_command_moves_between_subtasks_across_parents() {
    // Arrange
    let (mut session, file_uri) = super::helpers::start_project_session("");
    let file_content = "\
- [ ] parent A
  - [x] child A done
  - [ ] child A open
- [ ] parent B
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act: cursor on line 0 (parent A) -> the next actionable unit in parent
    // A's own subtree is its still-open child (line 2), since parent A has
    // remaining descendant work and can't be the target itself yet.
    let (show_document, _response) = session.execute_command(
        2,
        "mdagile.jump.nextOpen",
        vec![serde_json::json!(file_uri), serde_json::json!(0)],
    );
    assert_eq!(show_document["params"]["selection"]["start"]["line"], 2);

    // Act: from line 2, the next actionable unit is parent B (line 3) —
    // parent A has no more descendant work of its own now the cursor moved
    // past its only open child.
    let (show_document, _response) = session.execute_command(
        3,
        "mdagile.jump.nextOpen",
        vec![serde_json::json!(file_uri), serde_json::json!(2)],
    );

    // Assert
    assert_eq!(show_document["params"]["selection"]["start"]["line"], 3);
}

#[test]
fn lsp_jump_previous_open_command_moves_between_subtasks_across_parents() {
    // Arrange
    let (mut session, file_uri) = super::helpers::start_project_session("");
    let file_content = "\
- [ ] parent A
  - [x] child A done
  - [ ] child A open
- [ ] parent B
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act: cursor on line 3 (parent B) -> the nearest previous actionable
    // unit is parent A's open child (line 2), not parent A's own line.
    let (show_document, _response) = session.execute_command(
        2,
        "mdagile.jump.previousOpen",
        vec![serde_json::json!(file_uri), serde_json::json!(3)],
    );

    // Assert
    assert_eq!(show_document["params"]["selection"]["start"]["line"], 2);
}

#[test]
fn lsp_goto_implementation_jumps_into_subtask_eligible_via_inherited_assignment() {
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

    // The parent assigned to bob has no eligible work for alice at all, and
    // must be skipped entirely (an explicit assignment on a parent claims
    // its whole subtree) even though its own child carries no marker.
    // The unassigned parent's own unassigned subtask is the correct target.
    let file_content = "\
- [ ] parent for bob @bob
  - [ ] child with no marker of its own
- [ ] unassigned parent
  - [ ] unassigned child
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

    // Assert: line 3 (the unassigned parent's unassigned child), not line 1
    // (bob's child), and not either parent line (both have open descendant
    // work).
    assert_eq!(
        response["result"]["range"]["start"]["line"], 3,
        "GoTo Implementation should skip bob's whole subtree and land on line 3, got: {response}"
    );
}

#[test]
fn lsp_goto_implementation_skips_ordered_subtask_blocked_by_incomplete_predecessor() {
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
";
    fs::write(project_root.path().join("mdagile.toml"), config_toml).unwrap();

    let file_content = "\
- [ ] ordered parent @alice
  - [ ] 1. first step
  - [ ] 2. second step
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

    // Assert: "2. second step" (line 2) is order-blocked by the still-open
    // "1. first step" (line 1), so the actionable unit is the first step.
    assert_eq!(
        response["result"]["range"]["start"]["line"], 1,
        "GoTo Implementation should land on the first (unblocked) ordered step, got: {response}"
    );
}
