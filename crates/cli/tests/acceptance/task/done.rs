use crate::helpers::run_agile;
use std::fs;
use tempfile::tempdir;

#[test]
fn task_done_marks_top_level_task_complete_in_place() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] first task
- [ ] second task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "1"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    let expected = "\
- [x] first task
- [ ] second task
";
    assert_eq!(new_content, expected);
}

#[test]
fn task_done_marks_specific_subtask_via_dotted_address() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
  - [ ] subtask two
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "1.2"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    let expected = "\
- [ ] parent task
  - [ ] subtask one
  - [x] subtask two
";
    assert_eq!(new_content, expected);
}

#[test]
fn task_done_refuses_when_a_required_child_is_incomplete() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "1"]);

    assert!(!out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E004"), "stdout: {stdout:?}");
    // File must be left untouched.
    let new_content = fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap();
    assert_eq!(new_content, content);
}

#[test]
fn task_done_refuses_when_a_required_subtask_is_missing() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.needs_review]
subtasks = [\"code review\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] #needs_review parent task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "1"]);

    assert!(!out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E010"), "stdout: {stdout:?}");
}

#[test]
fn task_done_invalid_address_exits_nonzero() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "5"]);

    assert!(!out.status.success());
}

#[test]
fn task_done_on_already_done_task_exits_nonzero() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] already done
- [ ] still open
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // Address 1 selects the first *incomplete* top-level task ("still
    // open"), so marking it done twice in a row should fail the second time.
    let out = run_agile(dir.path(), &["task", "done", "1"]);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);

    let out = run_agile(dir.path(), &["task", "done", "1"]);
    assert!(!out.status.success());
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = std::process::Command::new("git")
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

/// The bug this reproduces: `agile task done` used to only run the
/// "incomplete children"/"missing required subtasks"/"out-of-order"
/// completion rules, never the E013 "unauthorized completion" one — so
/// anyone could complete a task assigned to someone else through the CLI
/// (and, by extension, through the GUI board, which reuses this same
/// mechanism). It must now be refused exactly like `agile check` would flag
/// it after the fact.
#[test]
fn task_done_refuses_to_complete_a_task_assigned_to_someone_else() {
    let dir = tempdir().unwrap();
    // Local git identity is "bob", but the task is assigned to alice only.
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

    let out = run_agile(dir.path(), &["task", "done", "1"]);

    assert!(!out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E013"), "stdout: {stdout:?}");
    // file must be untouched
    assert_eq!(
        fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap(),
        file_content
    );
}

#[test]
fn task_done_allows_completion_by_the_assigned_user() {
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

    let out = run_agile(dir.path(), &["task", "done", "1"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let expected = "\
- [x] fix bug @alice
";
    assert_eq!(
        fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap(),
        expected
    );
}

#[test]
fn task_done_as_flag_overrides_the_local_git_identity() {
    let dir = tempdir().unwrap();
    // Local git identity is "bob" (unassigned to this task), but `--as
    // alice` should authorize the completion regardless.
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

    let out = run_agile(dir.path(), &["task", "done", "1", "--as", "alice"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
}

#[test]
fn task_done_allows_completing_an_unassigned_task_outside_a_git_repo() {
    let dir = tempdir().unwrap();
    // Not a git repo at all — the identity resolves to "unrecognized", but
    // the task carries no assignment marker, so it stays open to anyone.
    let content = "\
- [ ] fix bug
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "1"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
}
