use super::helpers::{LspSession, file_uri};
use std::process::Command;

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git command failed to start");
    assert!(status.success(), "git {args:?} failed");
}

fn init_repo(dir: &std::path::Path, email: &str, name: &str) {
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", email]);
    git(dir, &["config", "user.name", name]);
}

fn commit_all(dir: &std::path::Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

#[test]
fn lsp_e013_reported_for_unauthorized_completion() {
    let project_root = tempfile::tempdir().unwrap();
    // "bob" is a known user, but the task below is assigned to alice only.
    init_repo(project_root.path(), "bob@example.com", "Bob");

    let config_toml = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    std::fs::write(project_root.path().join("mdagile.toml"), config_toml).unwrap();
    let file_content = "\
- [ ] fix bug @alice
";
    std::fs::write(project_root.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all(project_root.path(), "initial");

    let root_uri = file_uri(project_root.path());
    let task_file_uri = file_uri(&project_root.path().join("tasks.agile.md"));

    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(
        &task_file_uri,
        "\
- [x] fix bug @alice
",
    );

    let notification = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = notification["params"]["diagnostics"].as_array().unwrap();

    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"].as_str() == Some("E013")),
        "expected E013 for unauthorized completion, but got: {diagnostics:?}"
    );
}

#[test]
fn lsp_e013_not_reported_for_authorized_completion() {
    let project_root = tempfile::tempdir().unwrap();
    init_repo(project_root.path(), "alice@example.com", "Alice");

    let config_toml = "\
[Users.alice]
git_emails = [\"alice@example.com\"]
";
    std::fs::write(project_root.path().join("mdagile.toml"), config_toml).unwrap();
    let file_content = "\
- [ ] fix bug @alice
";
    std::fs::write(project_root.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all(project_root.path(), "initial");

    let root_uri = file_uri(project_root.path());
    let task_file_uri = file_uri(&project_root.path().join("tasks.agile.md"));

    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(
        &task_file_uri,
        "\
- [x] fix bug @alice
",
    );

    let notification = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = notification["params"]["diagnostics"].as_array().unwrap();

    assert!(
        !diagnostics
            .iter()
            .any(|d| d["code"].as_str() == Some("E013")),
        "expected no E013 for authorized completion, but got: {diagnostics:?}"
    );
}
