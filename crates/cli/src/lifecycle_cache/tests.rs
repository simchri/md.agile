use super::*;
use crate::parser;
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
fn completion_dates_for_uncommitted_close_is_unknown() {
    let dir = setup_repo();
    let file = dir.path().join("tasks.agile.md");

    let file_content = "\
- [ ] task a
";
    std::fs::write(&file, file_content).unwrap();
    commit_all_at(dir.path(), "c1", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
";
    std::fs::write(&file, file_content).unwrap();

    let current_content = std::fs::read_to_string(&file).unwrap();
    let current_items = parser::parse(&current_content, Path::new("tasks.agile.md").to_path_buf());
    let dates =
        completion_dates_for_current_file(dir.path(), Path::new("tasks.agile.md"), &current_items);
    assert!(dates.is_empty(), "dates: {dates:?}");
}
