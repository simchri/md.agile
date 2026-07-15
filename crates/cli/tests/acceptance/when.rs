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

    let out = run_agile(dir.path(), &["when", "--velocity"]);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    // The current implementation uses a fixed 90-day denominator:
    // one completed top-level task (weight 1) => 1/90 = 0.01 (2 decimals).
    assert_eq!(stdout, "0.01 weight/day\n", "stdout: {stdout:?}");
}
