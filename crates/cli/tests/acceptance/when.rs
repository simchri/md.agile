use crate::helpers::run_agile;
use std::fs;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git command failed to start");
    assert!(status.success(), "git {args:?} failed");
}

fn commit_all_at(dir: &std::path::Path, message: &str, iso_timestamp: &str) {
    git(dir, &["add", "-A"]);
    let status = Command::new("git")
        .args(["commit", "-q", "-m", message])
        .current_dir(dir)
        .env("GIT_AUTHOR_DATE", iso_timestamp)
        .env("GIT_COMMITTER_DATE", iso_timestamp)
        .status()
        .expect("git command failed to start");
    assert!(status.success(), "git commit at {iso_timestamp:?} failed");
}

fn assert_velocity(dir: &std::path::Path, expected_stdout: &str) {
    assert_velocity_with_args(dir, &["when", "--velocity"], expected_stdout);
}

fn assert_velocity_with_args(dir: &std::path::Path, args: &[&str], expected_stdout: &str) {
    let out = run_agile(dir, args);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout, expected_stdout, "stdout: {stdout:?}");
}

fn unix_ts_days_ago(days: u64) -> i64 {
    (SystemTime::now() - Duration::from_secs(days * 24 * 60 * 60))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn git_date_from_unix_secs(ts: i64) -> String {
    format!("{ts} +0000")
}

#[test]
fn when_velocity_prints_unknown_when_velocity_cannot_be_estimated() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["when", "--velocity"]);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout, "unknown\n", "stdout: {stdout:?}");
}

#[test]
fn when_velocity_includes_uncommitted_worktree_state_as_latest() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(2));
    let file_content = "\
- [ ] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    // Keep this change uncommitted: velocity should still include it.
    let file_content = "\
- [x] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // 1 completion over ~2 days (using worktree as latest state).
    assert_velocity(dir.path(), "0.50 weight/day\n");
}

#[test]
fn when_velocity_prints_weight_per_day_with_two_decimals() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task", "2026-07-11T12:00:00Z");

    // One completed top-level task (weight 1) over a 1-day commit span.
    assert_velocity(dir.path(), "1.00 weight/day\n");
}

#[test]
fn when_velocity_counts_direct_subtask_completion_with_half_weight() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] parent
  - [ ] child
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [ ] parent
  - [x] child
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete child", "2026-07-11T12:00:00Z");

    // A level-2 subtask contributes weight 1/2 over a 1-day span.
    assert_velocity(dir.path(), "0.50 weight/day\n");
}

#[test]
fn when_velocity_counts_nested_subtask_completion_with_depth_weight() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] parent
  - [ ] child
    - [ ] grandchild
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [ ] parent
  - [ ] child
    - [x] grandchild
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete grandchild", "2026-07-11T12:00:00Z");

    // A level-3 subtask contributes weight 1/3 over a 1-day span.
    assert_velocity(dir.path(), "0.33 weight/day\n");
}

#[test]
fn when_velocity_reordering_done_and_todo_tasks_does_not_increase_velocity() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [x] done task
- [ ] todo task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [ ] todo task
- [x] done task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "reorder only", "2026-07-11T12:00:00Z");

    assert_velocity(dir.path(), "0.00 weight/day\n");
}

#[test]
fn when_velocity_reordering_done_and_todo_tasks_preserves_nonzero_velocity() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] task a
- [ ] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task a", "2026-07-11T12:00:00Z");

    let file_content = "\
- [ ] task b
- [x] task a
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(
        dir.path(),
        "reorder after completion",
        "2026-07-12T12:00:00Z",
    );

    // 1 completion over a 2-day span; reordering later must not add velocity.
    assert_velocity(dir.path(), "0.50 weight/day\n");
}

#[test]
fn when_velocity_counts_completion_when_another_task_reopens_in_same_commit() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] task a
- [x] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(
        dir.path(),
        "complete a and reopen b",
        "2026-07-11T12:00:00Z",
    );

    // One completion (task a) over a 1-day span; reopening task b must not
    // cancel out the completion for velocity.
    assert_velocity(dir.path(), "1.00 weight/day\n");
}

#[test]
fn when_velocity_deleting_done_tasks_does_not_change_velocity() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [x] done task
- [ ] todo task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [ ] todo task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "delete done task", "2026-07-11T12:00:00Z");

    assert_velocity(dir.path(), "0.00 weight/day\n");
}

#[test]
fn when_velocity_deleting_done_tasks_preserves_nonzero_velocity() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] task a
- [ ] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task a", "2026-07-11T12:00:00Z");

    let file_content = "\
- [ ] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(
        dir.path(),
        "delete completed task a",
        "2026-07-12T12:00:00Z",
    );

    // 1 completion over a 2-day observed span; deleting the already-done task
    // later must not alter that completion history.
    assert_velocity(dir.path(), "0.50 weight/day\n");
}

#[test]
fn when_velocity_editing_title_of_done_task_does_not_change_velocity() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [x] done task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] renamed done task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "rename done task", "2026-07-11T12:00:00Z");

    assert_velocity(dir.path(), "0.00 weight/day\n");
}

#[test]
fn when_velocity_counts_real_completion_only_once_even_if_moved_later() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] task a
- [ ] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task a", "2026-07-11T12:00:00Z");

    let file_content = "\
- [ ] task b
- [x] task a
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "move completed task a", "2026-07-12T12:00:00Z");

    // 1 completion over a 2-day observed span.
    assert_velocity(dir.path(), "0.50 weight/day\n");
}

#[test]
fn when_velocity_same_timestamp_span_yields_unknown() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task", "2026-07-10T12:00:00Z");

    assert_velocity(dir.path(), "unknown\n");
}

#[test]
fn when_velocity_last_flag_restricts_history_window() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(6));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(4));
    let t2 = git_date_from_unix_secs(unix_ts_days_ago(1));

    let file_content = "\
- [ ] task a
- [ ] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [x] task a
- [ ] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete a", &t1);

    let file_content = "\
- [x] task a
- [x] task b
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete b", &t2);

    // Default window now reports the linear done-trend slope over the observed span.
    assert_velocity(dir.path(), "0.39 weight/day\n");
    // Restricting to last 2 days considers only the recent completion window.
    assert_velocity_with_args(
        dir.path(),
        &["when", "--velocity", "--last", "2"],
        "1.00 weight/day\n",
    );
}

#[test]
fn when_last_requires_velocity() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["when", "--last", "2"]);

    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("--velocity"),
        "expected clap error mentioning --velocity requirement, stderr: {stderr:?}"
    );
}
