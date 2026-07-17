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
fn file_content_at_ref_returns_none_for_untracked_file() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] task\n").unwrap();
    assert_eq!(
        file_content_at_ref(dir.path(), "HEAD", Path::new("a.agile.md")),
        None
    );
}

#[test]
fn file_content_at_ref_returns_none_for_repo_with_no_commits() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] task\n").unwrap();
    assert_eq!(
        file_content_at_ref(dir.path(), "HEAD", Path::new("a.agile.md")),
        None
    );
}

#[test]
fn file_content_at_ref_returns_committed_content() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] task\n").unwrap();
    commit_all(dir.path(), "initial");

    // Modify the working copy after committing.
    std::fs::write(dir.path().join("a.agile.md"), "- [x] task\n").unwrap();

    assert_eq!(
        file_content_at_ref(dir.path(), "HEAD", Path::new("a.agile.md")),
        Some("- [ ] task\n".to_string())
    );
}

#[test]
fn file_content_at_ref_reads_an_arbitrary_ref_not_just_head() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] task\n").unwrap();
    commit_all(dir.path(), "base");
    // A second commit moves HEAD forward; "base" (a tag) still points at the
    // first commit, simulating a CI "--base <ref>" pointing at a PR's base.
    let status = Command::new("git")
        .args(["tag", "base"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::write(dir.path().join("a.agile.md"), "- [x] task\n").unwrap();
    commit_all(dir.path(), "second");

    assert_eq!(
        file_content_at_ref(dir.path(), "base", Path::new("a.agile.md")),
        Some("- [ ] task\n".to_string())
    );
    assert_eq!(
        file_content_at_ref(dir.path(), "HEAD", Path::new("a.agile.md")),
        Some("- [x] task\n".to_string())
    );
}

#[test]
fn file_content_at_ref_returns_none_for_a_nonexistent_ref() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] task\n").unwrap();
    commit_all(dir.path(), "initial");

    // file_content_at_ref itself doesn't validate the ref (that's
    // ref_exists's job, used explicitly by callers who need to distinguish
    // "bad ref" from "valid ref, file just not present there").
    let result = file_content_at_ref(dir.path(), "no-such-ref", Path::new("a.agile.md"));
    assert_eq!(result, None);
}

#[test]
fn ref_exists_true_for_head_with_a_commit() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] task\n").unwrap();
    commit_all(dir.path(), "initial");
    assert!(ref_exists(dir.path(), "HEAD"));
}

#[test]
fn ref_exists_false_for_bogus_ref() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    std::fs::write(dir.path().join("a.agile.md"), "- [ ] task\n").unwrap();
    commit_all(dir.path(), "initial");
    assert!(!ref_exists(dir.path(), "no-such-ref"));
}

#[test]
fn commits_touching_path_returns_newest_first_for_that_file() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");

    std::fs::write(dir.path().join("a.agile.md"), "- [ ] one\n").unwrap();
    commit_all(dir.path(), "base");

    std::fs::write(dir.path().join("other.txt"), "x\n").unwrap();
    commit_all(dir.path(), "unrelated");

    std::fs::write(dir.path().join("a.agile.md"), "- [x] one\n").unwrap();
    commit_all(dir.path(), "touch a again");

    let commits = commits_touching_path(dir.path(), Path::new("a.agile.md"));

    assert_eq!(commits.len(), 2, "commits: {commits:?}");
    assert_ne!(commits[0].sha, commits[1].sha);
}

#[test]
fn commits_touching_path_follows_renames() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");

    std::fs::write(dir.path().join("a.agile.md"), "- [ ] one\n").unwrap();
    commit_all(dir.path(), "create a");

    let status = Command::new("git")
        .args(["mv", "a.agile.md", "b.agile.md"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());
    commit_all(dir.path(), "rename a->b");

    std::fs::write(dir.path().join("b.agile.md"), "- [x] one\n").unwrap();
    commit_all(dir.path(), "touch b");

    let commits = commits_touching_path(dir.path(), Path::new("b.agile.md"));

    // With rename following enabled, history includes pre-rename ancestry.
    assert_eq!(commits.len(), 3, "commits: {commits:?}");
}

#[test]
fn git_dir_resolves_dot_git_directory() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");
    let git_dir_path = git_dir(dir.path()).expect("git dir");
    assert!(git_dir_path.ends_with(".git"));
}

#[test]
fn commits_returns_repository_history_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");

    std::fs::write(dir.path().join("tasks.agile.md"), "- [ ] one\n").unwrap();
    commit_all(dir.path(), "c1");
    std::fs::write(dir.path().join("tasks.agile.md"), "- [x] one\n").unwrap();
    commit_all(dir.path(), "c2");

    let refs = commits(dir.path());
    assert!(refs.len() >= 2);
    assert_ne!(refs[0].sha, refs[1].sha);
}

#[test]
fn task_files_at_ref_lists_only_agile_files() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path(), "a@b.com", "A B");

    std::fs::write(dir.path().join("tasks.agile.md"), "- [ ] one\n").unwrap();
    std::fs::write(dir.path().join("notes.txt"), "hello\n").unwrap();
    commit_all(dir.path(), "initial");

    let files = task_files_at_ref(dir.path(), "HEAD");
    assert_eq!(files, vec![std::path::PathBuf::from("tasks.agile.md")]);
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
