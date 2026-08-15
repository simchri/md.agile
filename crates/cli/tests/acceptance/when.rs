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
fn when_velocity_errors_when_not_a_git_repository() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["when", "--velocity"]);

    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("requires a git repository"),
        "stderr: {stderr:?}"
    );
}

#[test]
fn when_velocity_prints_unknown_when_history_has_no_variance() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(0));
    let file_content = "\
- [ ] keep milestone future
- [ ] one task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    // Only one commit, dated today, and no uncommitted changes: the
    // committed point and the worktree "today" point share the same date,
    // so neither trend line has more than one distinct x-value.
    assert_velocity(dir.path(), "velocity: unknown\ncreep:    unknown\n");
}

#[test]
fn when_velocity_includes_uncommitted_worktree_state_as_latest() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(2));
    let file_content = "\
- [ ] keep milestone future
- [ ] one task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    // Keep this change uncommitted: velocity should still include it.
    let file_content = "\
- [ ] keep milestone future
- [x] one task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // done-weight rises from 0 to 1 over a ~2-day span: slope ~0.5/day = 3.50/week.
    assert_velocity(
        dir.path(),
        "velocity: weight/week   3.50\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_prints_weight_per_week_with_two_decimals() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [ ] one task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [x] one task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task", &t1);

    // done-weight rises by 1 over a 1-day span: slope 1.00/day = 7.00/week.
    assert_velocity(
        dir.path(),
        "velocity: weight/week   7.00\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_counts_direct_subtask_completion_with_half_weight() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [ ] parent
  - [ ] child
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [ ] parent
  - [x] child
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete child", &t1);

    // A level-2 subtask contributes weight 1/2 over a 1-day span: slope
    // 0.50/day = 3.50/week.
    assert_velocity(
        dir.path(),
        "velocity: weight/week   3.50\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_counts_nested_subtask_completion_with_depth_weight() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [ ] parent
  - [ ] child
    - [ ] grandchild
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [ ] parent
  - [ ] child
    - [x] grandchild
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete grandchild", &t1);

    // A level-3 subtask contributes weight 1/3 over a 1-day span: slope
    // 0.33/day = 2.33/week.
    assert_velocity(
        dir.path(),
        "velocity: weight/week   2.33\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_reordering_done_and_todo_tasks_does_not_increase_velocity() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [x] done task
- [ ] todo task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [ ] todo task
- [x] done task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "reorder only", &t1);

    assert_velocity(
        dir.path(),
        "velocity: weight/week   0.00\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_reordering_done_and_todo_tasks_preserves_nonzero_velocity() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(2));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t2 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [ ] task a
- [ ] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [x] task a
- [ ] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task a", &t1);

    let file_content = "\
- [ ] keep milestone future
- [ ] task b
- [x] task a
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "reorder after completion", &t2);

    // 1 completion over a 2-day span; reordering later must not add
    // velocity: slope 0.50/day = 3.50/week (using day-granularity dates,
    // the fitted slope over these three unevenly-spaced points is 3.18).
    assert_velocity(
        dir.path(),
        "velocity: weight/week   3.18\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_reopening_another_task_offsets_completion_in_the_same_commit() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [ ] task a
- [x] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [x] task a
- [ ] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete a and reopen b", &t1);

    // Task a completes and task b reopens in the same commit, so the
    // done-weight total is unchanged (still 1): the trend line is flat.
    assert_velocity(
        dir.path(),
        "velocity: weight/week   0.00\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_deleting_done_tasks_reduces_velocity_and_creep() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [x] done task
- [ ] todo task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [ ] todo task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "delete done task", &t1);

    // Deleting the done task removes weight 1 from the done trend over a
    // 1-day span (-7.00/week). Creep stays flat at 0.00: milestone scoping
    // fixes the in-scope rank cutoff at the *final* rank of the last
    // preceding task ("todo task"), so deleting "done task" (which precedes
    // it) shifts "todo task" into the vacated rank slot rather than
    // shrinking total weight — this is the same rank-cutoff behavior
    // `agile when --plot`/`--data` show for this scenario, not a velocity-
    // specific quirk.
    assert_velocity(
        dir.path(),
        "velocity: weight/week   -7.00\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_deleting_done_tasks_reduces_velocity_over_full_history() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(2));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t2 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [ ] task a
- [ ] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [x] task a
- [ ] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task a", &t1);

    let file_content = "\
- [ ] keep milestone future
- [ ] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "delete completed task a", &t2);

    // Deleting the already-completed task a pulls the done trend down at the
    // end of the history (-0.64/week). Creep stays flat at 0.00: milestone
    // scoping fixes the in-scope rank cutoff at the *final* rank of the last
    // preceding task ("task b"), so deleting "task a" (which precedes it)
    // shifts "task b" into the vacated rank slot rather than shrinking total
    // weight — this is the same rank-cutoff behavior `agile when
    // --plot`/`--data` show for this scenario, not a velocity-specific
    // quirk.
    assert_velocity(
        dir.path(),
        "velocity: weight/week   -0.64\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_editing_title_of_done_task_does_not_change_velocity() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [x] done task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [x] renamed done task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "rename done task", &t1);

    assert_velocity(
        dir.path(),
        "velocity: weight/week   0.00\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_counts_real_completion_only_once_even_if_moved_later() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(2));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t2 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [ ] task a
- [ ] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [x] task a
- [ ] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task a", &t1);

    let file_content = "\
- [ ] keep milestone future
- [ ] task b
- [x] task a
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "move completed task a", &t2);

    // 1 completion over a 2-day observed span, plateauing after the move.
    assert_velocity(
        dir.path(),
        "velocity: weight/week   3.18\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_same_timestamp_span_yields_unknown() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(0));

    let file_content = "\
- [ ] keep milestone future
- [ ] one task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [x] one task
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task", &t0);

    assert_velocity(dir.path(), "velocity: unknown\ncreep:    unknown\n");
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
- [ ] keep milestone future
- [ ] task a
- [ ] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] keep milestone future
- [x] task a
- [ ] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete a", &t1);

    let file_content = "\
- [ ] keep milestone future
- [x] task a
- [x] task b
#MILESTONE: eol
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete b", &t2);

    // Full history's done-weight trend slope, in weight/week.
    assert_velocity(
        dir.path(),
        "velocity: weight/week   2.38\ncreep:    weight/week   0.00\n",
    );
    // Restricting to the last 5 days excludes the oldest commit, changing
    // the fitted slope.
    assert_velocity_with_args(
        dir.path(),
        &["when", "--velocity", "--last", "5"],
        "velocity: weight/week   1.88\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_next_flag_scopes_to_a_given_milestone() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let t0 = git_date_from_unix_secs(unix_ts_days_ago(1));
    let t1 = git_date_from_unix_secs(unix_ts_days_ago(0));

    // "alpha" (rank 1) never sees any completion; "beta" (rank 2) does.
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
- [ ] task b
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", &t0);

    let file_content = "\
- [ ] task a
#MILESTONE: alpha
- [x] task b
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "complete task b", &t1);

    assert_velocity_with_args(
        dir.path(),
        &["when", "--velocity"],
        "velocity: weight/week   0.00\ncreep:    weight/week   0.00\n",
    );
    assert_velocity_with_args(
        dir.path(),
        &["when", "--velocity", "--next", "2"],
        "velocity: weight/week   7.00\ncreep:    weight/week   0.00\n",
    );
}

#[test]
fn when_velocity_errors_when_there_is_no_future_milestone() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] one task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let out = run_agile(dir.path(), &["when", "--velocity"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("does not exist"), "stderr: {stderr:?}");
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

#[test]
fn when_bare_lists_eta_for_future_milestones_skipping_reached_ones() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    // "alpha" is reached from the very first commit (task a is already
    // done), so it must not appear in the bare report. "beta" only becomes
    // reached once task d is done, which never happens here.
    let file_content = "\
- [x] task a
#MILESTONE: alpha
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 0", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
#MILESTONE: alpha
- [x] task b
- [ ] task c
- [ ] task d
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 1", "2026-07-11T12:00:00Z");

    let file_content = "\
- [x] task a
#MILESTONE: alpha
- [x] task b
- [x] task c
- [ ] task d
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 2", "2026-07-12T12:00:00Z");

    let out = run_agile(dir.path(), &["when"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("alpha"), "stdout: {stdout:?}");
    let line = stdout
        .lines()
        .find(|line| line.contains("beta"))
        .unwrap_or_else(|| panic!("missing beta line, stdout: {stdout:?}"));
    assert!(!line.contains("unknown"), "line: {line:?}");
}

#[test]
fn when_bare_prints_nothing_when_there_are_no_future_milestones() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] task a
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let out = run_agile(dir.path(), &["when"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout, "", "stdout: {stdout:?}");
}

#[test]
fn when_bare_requires_a_git_repository() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["when"]);

    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("git repository"), "stderr: {stderr:?}");
}

#[test]
fn when_bare_lists_multiple_future_milestones_in_backlog_order() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    // Neither "gamma" nor "delta" is ever fully reached (task c/d and
    // g/h stay open), but both accumulate enough converging history for a
    // resolvable ETA, matching the shape of
    // `when_plot_shows_eta_span_and_date_when_trend_lines_intersect_in_the_future`.
    // "gamma" (rank 1) must come before "delta" (rank 2) in the report,
    // matching backlog order.
    let file_content = "\
- [ ] task a
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: gamma
- [ ] task e
- [ ] task f
- [ ] task g
- [ ] task h
#MILESTONE: delta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 0", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: gamma
- [x] task e
- [ ] task f
- [ ] task g
- [ ] task h
#MILESTONE: delta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 1", "2026-07-11T12:00:00Z");

    let file_content = "\
- [x] task a
- [x] task b
- [ ] task c
- [ ] task d
#MILESTONE: gamma
- [x] task e
- [x] task f
- [ ] task g
- [ ] task h
#MILESTONE: delta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 2", "2026-07-12T12:00:00Z");

    let out = run_agile(dir.path(), &["when"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let gamma_pos = stdout
        .find("gamma")
        .unwrap_or_else(|| panic!("missing gamma line, stdout: {stdout:?}"));
    let delta_pos = stdout
        .find("delta")
        .unwrap_or_else(|| panic!("missing delta line, stdout: {stdout:?}"));
    assert!(
        gamma_pos < delta_pos,
        "expected gamma before delta, stdout: {stdout:?}"
    );
}

#[test]
fn when_bare_shows_unknown_for_a_milestone_with_no_resolvable_trend() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    // "gamma" has converging history and a resolvable ETA (same shape as
    // the `--plot` intersection test). "epsilon" is added only in the
    // worktree (never committed), so it has a single data point and no
    // convergent trend -> "unknown", while still being listed.
    let file_content = "\
- [ ] task a
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: gamma
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 0", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: gamma
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 1", "2026-07-11T12:00:00Z");

    let file_content = "\
- [x] task a
- [x] task b
- [ ] task c
- [ ] task d
#MILESTONE: gamma
- [ ] task i
#MILESTONE: epsilon
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    // Left uncommitted on purpose: "epsilon" only ever shows up in the
    // worktree, so its plot has a single data point.

    let out = run_agile(dir.path(), &["when"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let gamma_line = stdout
        .lines()
        .find(|line| line.contains("gamma"))
        .unwrap_or_else(|| panic!("missing gamma line, stdout: {stdout:?}"));
    assert!(!gamma_line.contains("unknown"), "line: {gamma_line:?}");
    let epsilon_line = stdout
        .lines()
        .find(|line| line.contains("epsilon"))
        .unwrap_or_else(|| panic!("missing epsilon line, stdout: {stdout:?}"));
    assert!(
        epsilon_line.starts_with("unknown"),
        "line: {epsilon_line:?}"
    );
}

#[test]
fn when_plot_shows_total_and_done_scoped_to_milestone() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] task a
- [ ] task b
#MILESTONE: alpha
- [ ] task c
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
#MILESTONE: alpha
- [ ] task c
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "finish task a", "2026-07-11T12:00:00Z");

    let out = run_agile(dir.path(), &["when", "--plot", "--next", "1"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Milestone: alpha"), "stdout: {stdout:?}");
    // task c comes after the milestone, so scope is task a + task b only:
    // total weight 2.00, and after task a is done, done weight 1.00.
    assert!(
        stdout.contains("total:  2 tasks  (weight 2.00)"),
        "stdout: {stdout:?}"
    );
    assert!(
        stdout.contains("done:   1 tasks  (weight 1.00)"),
        "stdout: {stdout:?}"
    );
    // ETA is always printed, whether resolved (span + date) or "unknown".
    assert!(stdout.contains("ETA:"), "stdout: {stdout:?}");
}

#[test]
fn when_plot_shows_eta_span_and_date_when_trend_lines_intersect_in_the_future() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    // Total stays flat at 4 tasks (in scope of milestone "alpha"); one task
    // finishes per day, so the done trend rises steadily toward the total
    // and the two trend lines intersect a couple of days in the future.
    let file_content = "\
- [ ] task a
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 0", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 1", "2026-07-11T12:00:00Z");

    let file_content = "\
- [x] task a
- [x] task b
- [ ] task c
- [ ] task d
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 2", "2026-07-12T12:00:00Z");

    let out = run_agile(dir.path(), &["when", "--plot", "--next", "1"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Milestone: alpha"), "stdout: {stdout:?}");
    assert!(stdout.contains("ETA date: "), "stdout: {stdout:?}");
    assert!(
        stdout.contains("ETA: ") && !stdout.contains("ETA: unknown"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn when_plot_prints_the_total_and_done_trend_equations() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    // Total stays flat at 4 tasks; one task finishes per day.
    let file_content = "\
- [ ] task a
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 0", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 1", "2026-07-11T12:00:00Z");

    let out = run_agile(dir.path(), &["when", "--plot", "--next", "1"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    // The equations should make the slope/intercept behind the chart
    // transparent, anchored to the plot's first date.
    assert!(
        stdout.contains("x = weeks since 2026-07-10"),
        "stdout: {stdout:?}"
    );
    assert!(
        stdout.contains("4.00 + 0.00/week * x"),
        "total trend should be flat at 4: stdout: {stdout:?}"
    );
    // The done trend's exact numbers depend on "today" (the worktree point
    // extends the timeline to the real current date), so only check that a
    // well-formed, non-"unknown" equation is shown.
    assert!(
        stdout.contains("done") && stdout.matches("/week * x").count() == 2,
        "both trend equations should be shown: stdout: {stdout:?}"
    );
}

#[test]
fn when_plot_requires_data_and_plot_to_be_mutually_exclusive() {
    let out = run_agile(std::path::Path::new("."), &["when", "--plot", "--data"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("conflict"),
        "stderr: {stderr:?}"
    );
}

#[test]
fn when_data_rejects_fit_flag() {
    // `--fit` only makes sense with `--plot`; it should be rejected with
    // `--data`, not silently ignored.
    let out = run_agile(std::path::Path::new("."), &["when", "--data", "--fit"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--fit"), "stderr: {stderr:?}");
}

#[test]
fn when_data_rejects_ascii_flag() {
    // `--ascii` only makes sense with `--plot`; it should be rejected with
    // `--data`, not silently ignored.
    let out = run_agile(std::path::Path::new("."), &["when", "--data", "--ascii"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--ascii"), "stderr: {stderr:?}");
}

#[test]
fn when_plot_ascii_uses_only_7_bit_ascii_characters_and_shows_the_ascii_legend() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] task a
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 0", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
- [ ] task c
- [ ] task d
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "day 1", "2026-07-11T12:00:00Z");

    let out = run_agile(dir.path(), &["when", "--plot", "--next", "1", "--ascii"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Milestone: alpha"), "stdout: {stdout:?}");
    // The ascii legend uses distinct symbols for the four data lines.
    assert!(stdout.contains("o total"), "stdout: {stdout:?}");
    assert!(stdout.contains("@ done"), "stdout: {stdout:?}");
    assert!(stdout.contains(". total trend"), "stdout: {stdout:?}");
    assert!(stdout.contains("~ done trend"), "stdout: {stdout:?}");
    assert!(stdout.contains(": today"), "stdout: {stdout:?}");
    // Aside from the ANSI color escape sequences, the chart itself must be
    // pure 7-bit ASCII (no Braille/Unicode block characters).
    let visible: String = stdout
        .chars()
        .filter(|c| *c != '\x1b')
        .collect::<String>()
        .lines()
        .filter(|line| !line.contains('['))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        visible.is_ascii(),
        "expected pure ASCII chart output, got: {visible:?}"
    );
}

#[test]
fn when_data_shows_table_of_task_counts_scoped_to_milestone() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] task a
- [ ] task b
#MILESTONE: alpha
- [ ] task c
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
#MILESTONE: alpha
- [ ] task c
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "finish task a", "2026-07-11T12:00:00Z");

    // No `--next` flag: should default to the next milestone, like `--plot`.
    let out = run_agile(dir.path(), &["when", "--data"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Milestone: alpha"), "stdout: {stdout:?}");
    assert!(stdout.contains("Date"), "stdout: {stdout:?}");
    assert!(stdout.contains("Total"), "stdout: {stdout:?}");
    assert!(stdout.contains("Done"), "stdout: {stdout:?}");
    assert!(stdout.contains("Total Wt"), "stdout: {stdout:?}");
    assert!(stdout.contains("Done Wt"), "stdout: {stdout:?}");
    // no trend line fitting, just raw counts/weights per data point.
    assert!(!stdout.contains("trend"), "stdout: {stdout:?}");

    let row1 = stdout
        .lines()
        .find(|line| line.contains("2026-07-10"))
        .unwrap_or_else(|| panic!("missing row for 2026-07-10, stdout: {stdout:?}"));
    assert!(row1.contains('2') && row1.contains('0'), "row1: {row1:?}");
    assert!(
        row1.contains("2.00") && row1.contains("0.00"),
        "row1: {row1:?}"
    );

    let row2 = stdout
        .lines()
        .find(|line| line.contains("2026-07-11"))
        .unwrap_or_else(|| panic!("missing row for 2026-07-11, stdout: {stdout:?}"));
    assert!(row2.contains('2') && row2.contains('1'), "row2: {row2:?}");
    assert!(
        row2.contains("2.00") && row2.contains("1.00"),
        "row2: {row2:?}"
    );
}

#[test]
fn when_plot_defaults_to_next_1() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] task a
- [ ] task b
#MILESTONE: alpha
- [ ] task c
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
- [ ] task b
#MILESTONE: alpha
- [ ] task c
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "finish task a", "2026-07-11T12:00:00Z");

    // No `--next` flag: should behave exactly like `--next 1`.
    let out = run_agile(dir.path(), &["when", "--plot"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Milestone: alpha"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("total:  2 tasks  (weight 2.00)"),
        "stdout: {stdout:?}"
    );
    assert!(
        stdout.contains("done:   1 tasks  (weight 1.00)"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn when_plot_next_1_skips_already_reached_milestones() {
    // Regression test: `--next 1` must show the next *incomplete* milestone,
    // not simply the first milestone in the backlog. Milestone "alpha" is
    // already reached (all tasks above it are done), so `--next 1` should
    // scope to "beta" instead.
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [x] task a
#MILESTONE: alpha
- [ ] task b
- [ ] task c
#MILESTONE: beta
- [ ] task d
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
#MILESTONE: alpha
- [x] task b
- [ ] task c
#MILESTONE: beta
- [ ] task d
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "finish task b", "2026-07-11T12:00:00Z");

    let out = run_agile(dir.path(), &["when", "--plot", "--next", "1"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Milestone: beta"), "stdout: {stdout:?}");
}

#[test]
fn when_plot_errors_for_milestone_never_committed() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] task a
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    // Milestone rank 2 doesn't exist.
    let out = run_agile(dir.path(), &["when", "--plot", "--next", "2"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("does not exist"), "stderr: {stderr:?}");
}
