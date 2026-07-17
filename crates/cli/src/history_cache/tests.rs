use super::*;
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git command failed to start");
    assert!(status.success(), "git {args:?} failed");
}

fn commit_all_at(dir: &Path, message: &str, git_date: &str) {
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

fn setup_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);
    dir
}

#[test]
fn update_appends_entry_for_new_commits() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task one
  - [ ] subtask
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    assert_eq!(cache.entries.len(), 1);
    assert_eq!(cache.entries[0].completion_events_from_previous, 0);
    assert!((cache.entries[0].completed_weight_from_previous - 0.0).abs() < f64::EPSILON);
    assert_eq!(cache.entries[0].open_tasks_count, 1);
    assert_eq!(cache.entries[0].done_tasks_count, 0);
    assert!((cache.entries[0].open_tasks_weight - 1.5).abs() < f64::EPSILON);

    let file_content = "\
- [x] task one
  - [x] subtask
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "done", "2026-07-11T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    assert_eq!(cache.entries.len(), 2);
    assert_eq!(cache.entries[1].completion_events_from_previous, 2);
    assert!((cache.entries[1].completed_weight_from_previous - 1.5).abs() < f64::EPSILON);
    assert_eq!(cache.entries[1].open_tasks_count, 0);
    assert_eq!(cache.entries[1].done_tasks_count, 1);
    assert!((cache.entries[1].done_tasks_weight - 1.5).abs() < f64::EPSILON);
}

#[test]
fn update_invalidates_changed_commit_and_following_entries() {
    let dir = setup_repo();
    let file_content = "\
- [ ] a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "c1", "2026-07-10T12:00:00Z");
    let first_sha = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    let file_content = "\
- [x] a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "c2", "2026-07-11T12:00:00Z");
    let old_second_sha = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    let cache = update(dir.path()).expect("cache");
    assert_eq!(cache.entries.len(), 2);
    assert_eq!(cache.entries[1].commit_hash, old_second_sha);

    git(dir.path(), &["reset", "--hard", &first_sha]);
    let file_content = "\
- [-] a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "c2-rewritten", "2026-07-12T12:00:00Z");
    let new_second_sha = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    assert_ne!(new_second_sha, old_second_sha);

    let cache = update(dir.path()).expect("cache");
    assert_eq!(cache.entries.len(), 2);
    assert_eq!(cache.entries[0].commit_hash, first_sha);
    assert_eq!(cache.entries[1].commit_hash, new_second_sha);
    assert_eq!(cache.entries[1].completion_events_from_previous, 0);
    assert!((cache.entries[1].completed_weight_from_previous - 0.0).abs() < f64::EPSILON);
    assert_eq!(cache.entries[1].done_tasks_count, 1);
}
