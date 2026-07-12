use crate::helpers::run_agile;
use std::fs;
use tempfile::tempdir;

#[test]
fn task_list_prints_active_tasks() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] first task
- [ ] second task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "list"]);

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] first task"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] second task"), "stdout: {stdout:?}");
}

#[test]
fn task_list_excludes_done_and_cancelled_by_default() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done task
- [-] cancelled task
- [ ] active task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "list"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !stdout.contains("[x]"),
        "done task should be excluded: {stdout:?}"
    );
    assert!(
        !stdout.contains("[-]"),
        "cancelled task should be excluded: {stdout:?}"
    );
    assert!(stdout.contains("[ ] active task"), "stdout: {stdout:?}");
}

#[test]
fn task_list_all_includes_done_and_cancelled() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done task
- [-] cancelled task
- [ ] active task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "list", "--all"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[x] done task"), "stdout: {stdout:?}");
    assert!(stdout.contains("[-] cancelled task"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] active task"), "stdout: {stdout:?}");
}

#[test]
fn task_list_includes_subtasks_in_output() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
  - [x] subtask done
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "list"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] parent task"), "stdout: {stdout:?}");
    assert!(stdout.contains("  [ ] subtask one"), "stdout: {stdout:?}");
    assert!(stdout.contains("  [x] subtask done"), "stdout: {stdout:?}");
}

#[test]
fn task_list_with_next_limit_returns_first_n_tasks() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] task one
- [ ] task two
- [ ] task three
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "list", "-n", "2"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] task one"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] task two"), "stdout: {stdout:?}");
    assert!(!stdout.contains("task three"), "stdout: {stdout:?}");
}

#[test]
fn task_list_with_last_limit_returns_last_n_tasks() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] task one
- [ ] task two
- [ ] task three
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "list", "--last", "1"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("task one"), "stdout: {stdout:?}");
    assert!(!stdout.contains("task two"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] task three"), "stdout: {stdout:?}");
}

#[test]
fn task_list_next_flag_takes_precedence_over_last() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] task one
- [ ] task two
- [ ] task three
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // --next 1 and --last 1 both set: --next wins, so we get the first task
    let out = run_agile(dir.path(), &["task", "list", "-n", "1", "--last", "1"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] task one"), "stdout: {stdout:?}");
    assert!(!stdout.contains("task three"), "stdout: {stdout:?}");
}

#[test]
fn task_list_empty_project_exits_zero_with_no_output() {
    let dir = tempdir().unwrap();

    let out = run_agile(dir.path(), &["task", "list"]);

    assert!(out.status.success());
    assert!(
        out.stdout.is_empty(),
        "expected no output: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn task_list_range_selects_inclusive_slice_with_subtasks() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] task one
  - [ ] one sub
- [ ] task two
- [ ] task three
  - [ ] three sub
- [ ] task four
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "list", "2:3"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("task one"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] task two"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] task three"), "stdout: {stdout:?}");
    assert!(stdout.contains("  [ ] three sub"), "stdout: {stdout:?}");
    assert!(!stdout.contains("task four"), "stdout: {stdout:?}");
}

#[test]
fn task_list_range_takes_precedence_over_next_and_last() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] task one
- [ ] task two
- [ ] task three
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(
        dir.path(),
        &["task", "list", "2:2", "-n", "1", "--last", "1"],
    );

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("task one"), "stdout: {stdout:?}");
    assert!(stdout.contains("[ ] task two"), "stdout: {stdout:?}");
    assert!(!stdout.contains("task three"), "stdout: {stdout:?}");
}

#[test]
fn task_list_invalid_range_exits_nonzero() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] task one
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "list", "4:2"]);

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

#[test]
fn task_list_mine_shows_unassigned_and_assigned_to_me_only() {
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
- [ ] alice's task @alice
- [ ] bob's task @bob
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "list", "--mine"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("unassigned task"), "stdout: {stdout:?}");
    assert!(stdout.contains("alice's task"), "stdout: {stdout:?}");
    assert!(!stdout.contains("bob's task"), "stdout: {stdout:?}");
}

#[test]
fn tasks_list_mine_works_the_same_as_task_list_mine_via_the_tasks_alias() {
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
- [ ] alice's task @alice
- [ ] bob's task @bob
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // "tasks" is a top-level alias for "task" (see Command::Task's
    // #[command(alias = "tasks")]) — confirm it works for "list" too.
    let out = run_agile(dir.path(), &["tasks", "list", "--mine"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("alice's task"), "stdout: {stdout:?}");
    assert!(!stdout.contains("bob's task"), "stdout: {stdout:?}");
}

#[test]
fn task_list_mine_with_as_override_uses_the_named_identity() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
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
- [ ] alice's task @alice
- [ ] bob's task @bob
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    // The local git identity is bob, but --as alice should switch the
    // filter to alice's assigned tasks instead.
    let out = run_agile(dir.path(), &["task", "list", "--mine", "--as", "alice"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("alice's task"), "stdout: {stdout:?}");
    assert!(!stdout.contains("bob's task"), "stdout: {stdout:?}");
}

#[test]
fn task_list_mine_combined_with_all_still_filters_by_eligibility() {
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
- [x] alice's done task @alice
- [x] bob's done task @bob
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "list", "--mine", "--all"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("alice's done task"), "stdout: {stdout:?}");
    assert!(!stdout.contains("bob's done task"), "stdout: {stdout:?}");
}

#[test]
fn task_list_mine_without_git_identity_errors() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "list", "--mine"]);

    assert!(!out.status.success());
}
