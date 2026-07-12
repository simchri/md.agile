use crate::helpers::run_agile;
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

#[test]
fn task_next_prints_first_active_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done task
- [ ] first active task
- [ ] second active task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "next"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("[ ] first active task"),
        "stdout: {stdout:?}"
    );
    assert!(
        !stdout.contains("second active"),
        "should only show first: {stdout:?}"
    );
}

#[test]
fn task_next_includes_subtree_of_next_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
  - [x] subtask done
- [ ] other task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "next"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] parent task"), "stdout: {stdout:?}");
    assert!(stdout.contains("  [ ] subtask one"), "stdout: {stdout:?}");
    assert!(stdout.contains("  [x] subtask done"), "stdout: {stdout:?}");
    assert!(!stdout.contains("other task"), "stdout: {stdout:?}");
}

#[test]
fn task_next_empty_project_exits_zero_with_no_output() {
    let dir = tempdir().unwrap();

    let out = run_agile(dir.path(), &["task", "next"]);

    assert!(out.status.success());
    assert!(
        out.stdout.is_empty(),
        "expected no output: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn tasks_alias_works_for_next() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] the task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["tasks", "next"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] the task"), "stdout: {stdout:?}");
}

#[test]
fn show_alias_works_bare_and_with_a_dotted_address() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] first task
  - [ ] subtask one
  - [ ] subtask two
- [ ] second task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let bare = run_agile(dir.path(), &["task", "show"]);
    assert!(bare.status.success());
    let bare_stdout = String::from_utf8(bare.stdout).unwrap();
    assert!(
        bare_stdout.contains("[ ] first task"),
        "stdout: {bare_stdout:?}"
    );

    let addressed = run_agile(dir.path(), &["task", "show", "1.2"]);
    assert!(addressed.status.success());
    let addressed_stdout = String::from_utf8(addressed.stdout).unwrap();
    assert!(
        addressed_stdout.contains("[ ] subtask two"),
        "stdout: {addressed_stdout:?}"
    );
    assert!(
        !addressed_stdout.contains("subtask one"),
        "stdout: {addressed_stdout:?}"
    );
}

#[test]
fn task_next_with_plain_number_shows_only_that_task_not_all_up_to_it() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done task
- [ ] first task
- [ ] second task
- [ ] third task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "next", "2"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !stdout.contains("first task"),
        "should not show task #1: {stdout:?}"
    );
    assert!(stdout.contains("second task"), "stdout: {stdout:?}");
    assert!(
        !stdout.contains("third task"),
        "should not show task #3: {stdout:?}"
    );
}

#[test]
fn task_next_with_dotted_address_shows_specific_subtask_as_its_own_root() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
    - [ ] grandchild
  - [ ] subtask two
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "next", "1.2"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("[ ] subtask two"),
        "should show subtask two as root: {stdout:?}"
    );
    assert!(
        !stdout.contains("parent task") && !stdout.contains("subtask one"),
        "should not show unrelated nodes: {stdout:?}"
    );
}

#[test]
fn task_next_with_deeper_dotted_address_descends_multiple_levels() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
    - [ ] grandchild a
    - [ ] grandchild b
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "next", "1.1.2"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] grandchild b"), "stdout: {stdout:?}");
    assert!(!stdout.contains("grandchild a"), "stdout: {stdout:?}");
}

#[test]
fn task_next_invalid_address_exits_nonzero() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "next", "1.99"]);

    assert!(!out.status.success());
}

#[test]
fn task_next_malformed_address_exits_nonzero() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "next", "abc"]);

    assert!(!out.status.success());
}

#[test]
fn task_next_mine_with_plain_number_selects_nth_eligible_task_only() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] unassigned task
- [ ] bob's task @bob
- [ ] alice's task @alice
- [ ] another unassigned task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // Eligible tasks (unassigned or alice's), in order: "unassigned task"
    // (#1), "alice's task" (#2), "another unassigned task" (#3) — bob's
    // task is skipped entirely, not just deprioritized. Address 2 should
    // therefore resolve to "alice's task" alone.
    let out = run_agile(dir.path(), &["task", "next", "2", "--mine"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("alice's task"), "stdout: {stdout:?}");
    assert!(!stdout.contains("unassigned task"), "stdout: {stdout:?}");
    assert!(!stdout.contains("bob's task"), "stdout: {stdout:?}");
}

#[test]
fn task_next_mine_with_dotted_address_is_rejected() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "next", "1.1", "--mine"]);

    assert!(!out.status.success());
}

#[test]
fn task_next_bare_mine_shows_only_first_eligible_task() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] bob's task @bob
- [ ] alice's task @alice
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "next", "--mine"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("alice's task"), "stdout: {stdout:?}");
    assert!(!stdout.contains("bob's task"), "stdout: {stdout:?}");
}

#[test]
fn task_next_as_without_mine_still_filters_by_the_named_identity() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    // Local git identity is bob, but `--as alice` (without `--mine`) should
    // still filter to alice's eligible tasks — `--as` implies `--mine`.
    git(dir.path(), &["config", "user.email", "bob@example.com"]);
    git(dir.path(), &["config", "user.name", "Bob"]);

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] bob's task @bob
- [ ] alice's task @alice
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "next", "--as", "alice"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("alice's task"), "stdout: {stdout:?}");
    assert!(!stdout.contains("bob's task"), "stdout: {stdout:?}");
}
