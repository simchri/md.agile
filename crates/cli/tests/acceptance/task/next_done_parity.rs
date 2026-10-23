//! Cross-command acceptance tests asserting `agile task next` and
//! `agile task done` agree on which (sub)task is "the next eligible one"
//! when no ADDRESS is given.
//!
//! `done.rs` and `next.rs`/`next_eligibility.rs` each separately exercise
//! this no-address selection logic, but — aside from the single `<N>
//! --mine` rank-parity test in `next.rs`
//! (`task_next_mine_prints_the_rank_not_the_eligible_only_position`) — no
//! existing test runs *both* commands against the *same* fixture and checks
//! they target the identical node. These tests close that gap: for a given
//! fixture, `agile task next --no-markup` is run first (read-only) to
//! determine which line it highlights with `" <=="`, then `agile task
//! done` is run against the same file and the resulting diff is checked
//! against that same line.

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

fn init_repo(dir: &std::path::Path, email: &str, name: &str) {
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", email]);
    git(dir, &["config", "user.name", name]);
}

fn stdout_of(dir: &std::path::Path, args: &[&str]) -> String {
    let out = run_agile(dir, args);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn plain_no_address_next_and_done_target_the_same_top_level_task() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] first task
- [ ] second task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let next_stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);
    assert!(
        next_stdout.contains("[ ] first task <=="),
        "stdout: {next_stdout:?}"
    );

    let out = run_agile(dir.path(), &["task", "done"]);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);

    let expected = "\
- [x] first task
- [ ] second task
";
    assert_eq!(
        fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap(),
        expected,
        "`task done` must flip exactly the task `task next` highlighted"
    );
}

#[test]
fn plain_no_address_next_and_done_target_the_same_nested_subtask() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [x] subtask one
  - [ ] subtask two
    - [ ] grandchild
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let next_stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);
    assert!(
        next_stdout.contains("[ ] grandchild <=="),
        "stdout: {next_stdout:?}"
    );

    let out = run_agile(dir.path(), &["task", "done"]);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);

    let expected = "\
- [ ] parent task
  - [x] subtask one
  - [ ] subtask two
    - [x] grandchild
";
    assert_eq!(
        fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap(),
        expected,
        "`task done` must flip exactly the grandchild `task next` highlighted"
    );
}

#[test]
fn mine_no_address_next_and_done_target_the_same_eligible_task() {
    let dir = tempdir().unwrap();
    init_repo(dir.path(), "alice@example.com", "Alice");

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] fix bug @bob
- [ ] write docs @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let next_stdout = stdout_of(dir.path(), &["task", "next", "--mine", "--no-markup"]);
    assert!(
        next_stdout.contains("[ ] write docs @alice <=="),
        "stdout: {next_stdout:?}"
    );
    assert!(
        !next_stdout.contains("fix bug"),
        "`--mine` must skip bob's task entirely: {next_stdout:?}"
    );

    let out = run_agile(dir.path(), &["task", "done", "--mine"]);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);

    let expected = "\
- [ ] fix bug @bob
- [x] write docs @alice
";
    assert_eq!(
        fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap(),
        expected,
        "`task done --mine` must flip exactly the task `task next --mine` highlighted"
    );
}

#[test]
fn task_done_mine_without_git_identity_errors() {
    // Parity with `task list --mine` (see
    // `task_list_mine_without_git_identity_errors` in list.rs) and `task
    // next --mine`, both of which hard-fail rather than silently
    // proceeding when no identity can be resolved at all (not a git repo,
    // or no git identity configured) — `--mine` cannot filter by identity
    // without one.
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "done", "--mine"]);

    assert!(!out.status.success());
    // file must be untouched
    assert_eq!(
        fs::read_to_string(dir.path().join("tasks.agile.md")).unwrap(),
        file_content
    );
}
