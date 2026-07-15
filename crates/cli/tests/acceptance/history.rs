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

fn commit_all_at(dir: &std::path::Path, message: &str, git_date: &str) {
    git(dir, &["add", "-A"]);
    let status = Command::new("git")
        .args(["commit", "-q", "-m", message])
        .current_dir(dir)
        .env("GIT_AUTHOR_DATE", git_date)
        .env("GIT_COMMITTER_DATE", git_date)
        .status()
        .expect("git command failed to start");
    assert!(status.success(), "git commit at {git_date:?} failed");
}

#[test]
fn history_shows_closed_tasks_with_dates_and_unknown_when_not_determinable() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let file_content = "\
- [ ] foo
  - [ ] bar
- [ ] baz
- [ ] qux
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] foo
  - [x] bar
- [ ] baz
- [ ] qux
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "close foo and bar", "2026-07-11T12:00:00Z");

    let file_content = "\
- [x] foo
  - [x] bar
- [-] baz
- [ ] qux
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "cancel baz", "2026-07-12T12:00:00Z");

    // Uncommitted close remains unknown by design.
    let file_content = "\
- [x] foo
  - [x] bar
- [-] baz
- [x] qux
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["history"]);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("2026-07-11 - [x] foo"),
        "stdout: {stdout:?}"
    );
    assert!(
        stdout.contains("2026-07-11   - [x] bar"),
        "stdout: {stdout:?}"
    );
    assert!(
        stdout.contains("2026-07-12 - [-] baz"),
        "stdout: {stdout:?}"
    );
    assert!(stdout.contains("unknown - [x] qux"), "stdout: {stdout:?}");
}

#[test]
fn history_and_velocity_share_transition_logic_for_balanced_close_reopen_commit() {
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
        "close a and reopen b in one commit",
        "2026-07-11T12:00:00Z",
    );

    let out = run_agile(dir.path(), &["history"]);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("2026-07-11 - [x] task a"),
        "stdout: {stdout:?}"
    );
    assert!(!stdout.contains("task b"), "stdout: {stdout:?}");
}
