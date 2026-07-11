use crate::helpers::{run_agile, run_check};
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

#[test]
fn flags_completion_by_identity_not_in_config_at_all() {
    let dir = tempdir().unwrap();
    // The committer's identity ("mallory") isn't a [Users.X] entry at all,
    // unlike "flags_completion_by_unassigned_user" where the committer is a
    // known-but-unassigned user. An unrecognized identity is always
    // unauthorized for an assigned task, it's never silently skipped.
    init_repo(dir.path(), "mallory@example.com", "Mallory");

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

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E013"), "stdout: {stdout:?}");
}

#[test]
fn as_flag_overrides_the_local_git_identity() {
    let dir = tempdir().unwrap();
    // The local git identity is "bob" (unassigned), but `--as alice`
    // overrides it to the assigned user, so the completion is allowed.
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

    let out = run_agile(dir.path(), &["check", "--as", "alice"]);

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn as_flag_with_an_unknown_user_key_is_always_unauthorized() {
    let dir = tempdir().unwrap();
    // Even though the local git identity is the assigned user "alice",
    // `--as` names a key that isn't in the config at all. This must be
    // treated as unauthorized, not fall back to the local git identity and
    // not be treated as a CLI usage error.
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

    let out = run_agile(dir.path(), &["check", "--as", "mallory"]);

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E013"), "stdout: {stdout:?}");
}

#[test]
fn base_flag_compares_against_an_older_ref_than_head() {
    let dir = tempdir().unwrap();
    // Simulates a CI checkout: the PR branch is fully committed (so
    // HEAD-vs-working-copy alone would show no diff at all), and the
    // completion actually happened relative to the PR's base branch.
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
    git(dir.path(), &["branch", "main-base"]);

    let file_content = "\
- [x] fix bug @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all(dir.path(), "complete the task");

    let out = run_agile(dir.path(), &["check", "--base", "main-base"]);

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E013"), "stdout: {stdout:?}");
}

#[test]
fn base_flag_with_a_nonexistent_ref_is_a_hard_error() {
    let dir = tempdir().unwrap();
    init_repo(dir.path(), "alice@example.com", "Alice");

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [x] fix bug @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all(dir.path(), "initial");

    let out = run_agile(dir.path(), &["check", "--base", "no-such-ref"]);

    // A typo'd --base must never be silently ignored: it's a distinct
    // failure mode from an authorization issue, reported on stderr.
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("no-such-ref"), "stderr: {stderr:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E013"), "stdout: {stdout:?}");
}

#[test]
fn as_and_base_flags_combine_for_the_full_ci_use_case() {
    let dir = tempdir().unwrap();
    // The CI runner's own git identity ("ci-bot") is irrelevant; what
    // matters is the PR author ("bob", via --as) and the PR's base branch
    // (via --base), simulating checking a fully-committed PR branch.
    init_repo(dir.path(), "ci-bot@example.com", "CI Bot");

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
    git(dir.path(), &["branch", "main-base"]);

    let file_content = "\
- [x] fix bug @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all(dir.path(), "complete the task");

    let out = run_agile(dir.path(), &["check", "--as", "bob", "--base", "main-base"]);

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E013"), "stdout: {stdout:?}");
}

#[test]
fn warns_on_stderr_when_git_identity_cannot_be_determined() {
    let dir = tempdir().unwrap();
    // Commit with a real identity first (git requires one to commit at
    // all), then clear it to simulate a contributor who hasn't configured
    // `user.email`/`user.name` locally.
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
    git(dir.path(), &["config", "user.email", ""]);
    git(dir.path(), &["config", "user.name", ""]);

    let file_content = "\
- [x] fix bug @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    // The check itself is skipped (no identity to validate against), but
    // unlike the "not a git repo at all" case, this must be surfaced as a
    // warning rather than silently passing.
    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.to_lowercase().contains("warn"), "stderr: {stderr:?}");
    assert!(
        stderr.contains("identity"),
        "expected a mention of the undeterminable identity, stderr: {stderr:?}"
    );
}

#[test]
fn does_not_warn_on_stderr_when_simply_not_a_git_repo() {
    let dir = tempdir().unwrap();
    // No git repo at all: assignment validation isn't applicable here, so
    // no warning is expected (unlike the "git repo but no identity" case).

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
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(!stderr.to_lowercase().contains("warn"), "stderr: {stderr:?}");
}
