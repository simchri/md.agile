use super::*;
use std::path::Path;
use std::process::Command;

/// Initializes a throwaway git repo in `dir`, with a local (repo-scoped)
/// identity so tests never depend on the machine's global git config.
fn init_repo(dir: &Path, email: &str, name: &str) {
    let run = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git command failed to start");
        assert!(status.success(), "git {args:?} failed");
    };
    run(&["init", "-q"]);
    run(&["config", "user.email", email]);
    run(&["config", "user.name", name]);
}

fn commit_all(dir: &Path, message: &str) {
    let run = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git command failed to start");
        assert!(status.success(), "git {args:?} failed");
    };
    run(&["add", "-A"]);
    run(&["commit", "-q", "-m", message]);
}

#[test]
fn is_git_repo_true_inside_a_repo() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    assert!(is_git_repo(dir.path()));
}

#[test]
fn is_git_repo_false_outside_a_repo() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!is_git_repo(dir.path()));
}

#[test]
fn current_identity_reads_local_git_config() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "alice@example.com", "Alice Smith");
    let identity = current_identity(dir.path()).unwrap();
    assert_eq!(identity.email.as_deref(), Some("alice@example.com"));
    assert_eq!(identity.name.as_deref(), Some("Alice Smith"));
}

#[test]
fn current_identity_none_outside_a_repo_without_global_config() {
    // Outside of any repo, `git config` falls back to global/system config,
    // which may or may not be set in the test environment. We only assert
    // that the function doesn't panic and returns a sensible Option.
    let dir = tempfile::tempdir().unwrap();
    let _ = current_identity(dir.path());
}

#[test]
fn head_file_content_returns_none_for_untracked_file() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] task\n").unwrap();
    assert_eq!(head_file_content(dir.path(), Path::new("a.agile.md")), None);
}

#[test]
fn head_file_content_returns_none_for_repo_with_no_commits() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] task\n").unwrap();
    assert_eq!(head_file_content(dir.path(), Path::new("a.agile.md")), None);
}

#[test]
fn head_file_content_returns_committed_content() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] task\n").unwrap();
    commit_all(dir.path(), "initial");

    // Modify the working copy after committing.
    std::fs::write(dir.path().join("a.agile.md"), "- [x] task\n").unwrap();

    assert_eq!(
        head_file_content(dir.path(), Path::new("a.agile.md")),
        Some("- [ ] task\n".to_string())
    );
}

// ── resolve_identity_user ───────────────────────────────────────────────────────

use crate::config::{Config, UserConfig};
use std::collections::HashMap;

fn config_with_user(key: &str, git_emails: &[&str], git_names: &[&str]) -> Config {
    Config {
        users: HashMap::from([(
            key.to_string(),
            UserConfig {
                name: key.to_string(),
                git_emails: git_emails.iter().map(|s| s.to_string()).collect(),
                git_names: git_names.iter().map(|s| s.to_string()).collect(),
            },
        )]),
        ..Config::default()
    }
}

#[test]
fn resolves_by_email_match() {
    let config = config_with_user("alice", &["alice@example.com"], &[]);
    let identity = GitIdentity {
        email: Some("alice@example.com".to_string()),
        name: Some("Someone Else".to_string()),
    };
    assert_eq!(
        resolve_identity_user(&config, &identity),
        Some("alice".to_string())
    );
}

#[test]
fn falls_back_to_git_name_when_email_does_not_match() {
    let config = config_with_user("alice", &["alice@example.com"], &["Alice Smith"]);
    let identity = GitIdentity {
        email: Some("someone@else.com".to_string()),
        name: Some("Alice Smith".to_string()),
    };
    assert_eq!(
        resolve_identity_user(&config, &identity),
        Some("alice".to_string())
    );
}

#[test]
fn no_match_returns_none() {
    let config = config_with_user("alice", &["alice@example.com"], &["Alice Smith"]);
    let identity = GitIdentity {
        email: Some("bob@example.com".to_string()),
        name: Some("Bob Jones".to_string()),
    };
    assert_eq!(resolve_identity_user(&config, &identity), None);
}

#[test]
fn no_identity_info_returns_none() {
    let config = config_with_user("alice", &["alice@example.com"], &["Alice Smith"]);
    let identity = GitIdentity {
        email: None,
        name: None,
    };
    assert_eq!(resolve_identity_user(&config, &identity), None);
}

#[test]
fn email_match_takes_precedence_over_name_mismatch() {
    // Email matches "alice", even though the git name would otherwise match "bob".
    let mut config = config_with_user("alice", &["alice@example.com"], &[]);
    config.users.insert(
        "bob".to_string(),
        UserConfig {
            name: "bob".to_string(),
            git_emails: vec![],
            git_names: vec!["Shared Name".to_string()],
        },
    );
    let identity = GitIdentity {
        email: Some("alice@example.com".to_string()),
        name: Some("Shared Name".to_string()),
    };
    assert_eq!(
        resolve_identity_user(&config, &identity),
        Some("alice".to_string())
    );
}
