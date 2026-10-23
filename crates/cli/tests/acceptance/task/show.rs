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
fn task_show_with_no_address_shows_the_rank_1_task() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] first task
  - [ ] subtask one
  - [ ] subtask two
- [ ] second task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "show"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] first task"), "stdout: {stdout:?}");
}

#[test]
fn task_show_empty_project_exits_zero_with_no_output() {
    let dir = tempdir().unwrap();

    let out = run_agile(dir.path(), &["task", "show"]);

    assert!(out.status.success());
    assert!(
        out.stdout.is_empty(),
        "expected no output: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn task_show_with_plain_number_shows_only_that_rank_not_all_up_to_it() {
    let dir = tempdir().unwrap();
    let content = "\
- [x] done task
- [ ] first task
- [ ] second task
- [ ] third task
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "show", "2"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !stdout.contains("first task"),
        "should not show rank 1: {stdout:?}"
    );
    assert!(stdout.contains("second task"), "stdout: {stdout:?}");
    assert!(
        !stdout.contains("third task"),
        "should not show rank 3: {stdout:?}"
    );
}

#[test]
fn task_show_with_dotted_address_shows_specific_subtask_as_its_own_root() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
    - [ ] grandchild
  - [ ] subtask two
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "show", "1.2"]);

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
fn task_show_with_deeper_dotted_address_descends_multiple_levels() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] parent task
  - [ ] subtask one
    - [ ] grandchild a
    - [ ] grandchild b
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "show", "1.1.2"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[ ] grandchild b"), "stdout: {stdout:?}");
    assert!(!stdout.contains("grandchild a"), "stdout: {stdout:?}");
}

#[test]
fn task_show_ignores_eligibility_and_shows_a_task_assigned_to_someone_else() {
    // Unlike `next`/`list --mine`, `show` has no identity filter at all: it
    // must resolve and print a task assigned to someone else, exactly like
    // `agile task done`/`undone` do for their own ADDRESS.
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
";
    fs::write(dir.path().join("tasks.agile.md"), content).unwrap();

    let out = run_agile(dir.path(), &["task", "show", "1"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("bob's task"), "stdout: {stdout:?}");
}

#[test]
fn task_show_rejects_the_mine_flag() {
    // `show` has no `--mine`/`--as` at all — it's not a recognized flag.
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "show", "--mine"]);

    assert!(!out.status.success());
}

#[test]
fn task_show_out_of_range_address_exits_nonzero() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "show", "1.99"]);

    assert!(!out.status.success());
}

#[test]
fn task_show_highlights_the_next_actionable_leaf_within_a_dotted_addressed_subtree() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] addressed subtask
    - [x] already done grandchild
    - [ ] next leaf
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "show", "1.1", "--no-markup"]);

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    let expected = "\
1.1   [ ] addressed subtask
1.1.1   [x] already done grandchild
1.1.2   [ ] next leaf <==
";
    assert_eq!(stdout, expected);
}

#[test]
fn task_show_malformed_address_exits_nonzero() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["task", "show", "abc"]);

    assert!(!out.status.success());
}
