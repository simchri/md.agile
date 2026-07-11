use crate::helpers::run_check;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

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
fn flags_completion_by_unassigned_user() {
    let dir = tempdir().unwrap();
    // Committer identity is "bob", but the task is assigned to alice only.
    init_repo(dir.path(), "bob@example.com", "Bob");

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] fix bug @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all(dir.path(), "initial");

    let file_content = "\
- [x] fix bug @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E013"), "stdout: {stdout:?}");
}

#[test]
fn allows_completion_by_assigned_user() {
    let dir = tempdir().unwrap();
    init_repo(dir.path(), "alice@example.com", "Alice");

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] fix bug @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all(dir.path(), "initial");

    let file_content = "\
- [x] fix bug @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn allows_completion_of_unassigned_task_by_anyone() {
    let dir = tempdir().unwrap();
    init_repo(dir.path(), "bob@example.com", "Bob");

    let config = "\
[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] fix bug
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all(dir.path(), "initial");

    let file_content = "\
- [x] fix bug
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn allows_completion_via_authorized_group_membership() {
    let dir = tempdir().unwrap();
    init_repo(dir.path(), "carol@example.com", "Carol");

    let config = "\
[Users.carol]
git_emails = [\"carol@example.com\"]

[Groups.reviewers]
members = [\"carol\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] fix bug @reviewers
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all(dir.path(), "initial");

    let file_content = "\
- [x] fix bug @reviewers
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn skips_check_when_not_a_git_repo() {
    let dir = tempdir().unwrap();
    // No git repo, no git identity available: check should be silently skipped.

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [x] fix bug @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
