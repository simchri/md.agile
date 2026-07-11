use super::*;
use crate::config::Config;
use crate::parser::parse;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn run_returns_no_issues_for_clean_input() {
    let input = "\
- [ ] top
  - [ ] sub
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    assert!(run(&items, &Config::default()).is_empty());
}

#[test]
fn run_aggregates_rule_issues() {
    let input = "\
- [ ] top

  - [ ] orphan
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = run(&items, &Config::default());
    assert_eq!(issues.len(), 1);
}

// ── check_authorization ────────────────────────────────────────────────────────

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

fn config_with_alice() -> Config {
    Config::from_str(
        "\
[Users.alice]
git_emails = [\"alice@example.com\"]
",
    )
    .unwrap()
}

fn config_with_alice_and_bob() -> Config {
    Config::from_str(
        "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
",
    )
    .unwrap()
}

#[test]
fn check_authorization_flags_unauthorized_completion() {
    let dir = tempfile::tempdir().unwrap();
    // "bob" is a known user, but the task below is assigned to alice only.
    init_repo(dir.path(), "bob@example.com", "Bob");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] fix bug @alice\n").unwrap();
    commit_all(dir.path(), "initial");
    std::fs::write(dir.path().join("a.agile.md"), "- [x] fix bug @alice\n").unwrap();

    let config = config_with_alice_and_bob();
    let issues = check_authorization(dir.path(), &config, None, None).unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].code,
        crate::rules::ErrorCode::UnauthorizedCompletion
    );
}

#[test]
fn check_authorization_allows_authorized_completion() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "alice@example.com", "Alice");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] fix bug @alice\n").unwrap();
    commit_all(dir.path(), "initial");
    std::fs::write(dir.path().join("a.agile.md"), "- [x] fix bug @alice\n").unwrap();

    let config = config_with_alice();
    let issues = check_authorization(dir.path(), &config, None, None).unwrap();
    assert!(issues.is_empty());
}

#[test]
fn check_authorization_skipped_outside_git_repo() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.agile.md"), "- [x] fix bug @alice\n").unwrap();

    let config = config_with_alice();
    let issues = check_authorization(dir.path(), &config, None, None).unwrap();
    assert!(issues.is_empty());
}

#[test]
fn check_authorization_flags_unrecognized_identity_completing_assigned_task() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "unknown@example.com", "Unknown");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] fix bug @alice\n").unwrap();
    commit_all(dir.path(), "initial");
    std::fs::write(dir.path().join("a.agile.md"), "- [x] fix bug @alice\n").unwrap();

    let config = config_with_alice();
    let issues = check_authorization(dir.path(), &config, None, None).unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].code,
        crate::rules::ErrorCode::UnauthorizedCompletion
    );
}

#[test]
fn check_authorization_skips_unrecognized_identity_completing_unassigned_task() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "unknown@example.com", "Unknown");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] fix bug\n").unwrap();
    commit_all(dir.path(), "initial");
    std::fs::write(dir.path().join("a.agile.md"), "- [x] fix bug\n").unwrap();

    let config = config_with_alice();
    let issues = check_authorization(dir.path(), &config, None, None).unwrap();
    assert!(issues.is_empty());
}

// ── resolve_repo_identity skip reasons ──────────────────────────────────────

#[test]
fn resolve_repo_identity_reports_not_a_git_repo() {
    let dir = tempfile::tempdir().unwrap();
    let resolution = resolve_repo_identity(dir.path(), &config_with_alice(), None);
    assert!(matches!(resolution, IdentityResolution::NotAGitRepo));
}

#[test]
fn resolve_repo_identity_reports_no_git_identity_when_git_config_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    // Repo exists, but user.email/user.name are left unset (empty string
    // rather than absent, to make sure the "unset" case, not just "missing
    // key", is detected too).
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", ""]);
    git(dir.path(), &["config", "user.name", ""]);

    let resolution = resolve_repo_identity(dir.path(), &config_with_alice(), None);
    assert!(matches!(resolution, IdentityResolution::NoGitIdentity));
}

#[test]
fn resolve_repo_identity_determined_via_override_even_without_git_identity() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", ""]);
    git(dir.path(), &["config", "user.name", ""]);

    let resolution = resolve_repo_identity(dir.path(), &config_with_alice(), Some("alice"));
    assert!(matches!(
        resolution,
        IdentityResolution::Determined(ResolvedIdentity::Known(ref key)) if key == "alice"
    ));
}

#[test]
fn check_authorization_warns_when_git_identity_cannot_be_determined() {
    let dir = tempfile::tempdir().unwrap();
    // Commit with a real identity first (git requires one to commit at all),
    // then clear it to simulate a repo whose current user hasn't configured
    // `user.email`/`user.name`.
    init_repo(dir.path(), "alice@example.com", "Alice");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] fix bug @alice\n").unwrap();
    commit_all(dir.path(), "initial");
    git(dir.path(), &["config", "user.email", ""]);
    git(dir.path(), &["config", "user.name", ""]);
    std::fs::write(dir.path().join("a.agile.md"), "- [x] fix bug @alice\n").unwrap();

    let config = config_with_alice();
    let (issues, skip_reason) =
        check_authorization_with_skip_reason(dir.path(), &config, None, None).unwrap();
    assert!(issues.is_empty());
    assert!(matches!(
        skip_reason,
        Some(IdentityResolution::NoGitIdentity)
    ));
}

#[test]
fn check_authorization_does_not_warn_outside_git_repo() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.agile.md"), "- [x] fix bug @alice\n").unwrap();

    let config = config_with_alice();
    let (issues, skip_reason) =
        check_authorization_with_skip_reason(dir.path(), &config, None, None).unwrap();
    assert!(issues.is_empty());
    assert!(matches!(skip_reason, Some(IdentityResolution::NotAGitRepo)));
}

// ── check_authorization_for_document (LSP: single unsaved buffer) ──────────────

#[test]
fn for_document_flags_unauthorized_completion_in_unsaved_buffer() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "bob@example.com", "Bob");
    let path = dir.path().join("a.agile.md");
    std::fs::write(&path, "- [ ] fix bug @alice\n").unwrap();
    commit_all(dir.path(), "initial");
    // Buffer text (not yet saved to disk) already shows the task done.
    let buffer_text = "- [x] fix bug @alice\n";

    let config = config_with_alice_and_bob();
    let issues = check_authorization_for_document(dir.path(), &path, buffer_text, &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].code,
        crate::rules::ErrorCode::UnauthorizedCompletion
    );
}

#[test]
fn for_document_allows_authorized_completion() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "alice@example.com", "Alice");
    let path = dir.path().join("a.agile.md");
    std::fs::write(&path, "- [ ] fix bug @alice\n").unwrap();
    commit_all(dir.path(), "initial");
    let buffer_text = "- [x] fix bug @alice\n";

    let config = config_with_alice();
    let issues = check_authorization_for_document(dir.path(), &path, buffer_text, &config);
    assert!(issues.is_empty());
}

#[test]
fn for_document_skipped_outside_git_repo() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.agile.md");
    let buffer_text = "- [x] fix bug @alice\n";

    let config = config_with_alice();
    let issues = check_authorization_for_document(dir.path(), &path, buffer_text, &config);
    assert!(issues.is_empty());
}
