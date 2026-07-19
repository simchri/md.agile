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

fn entity_for_title<'a>(cache: &'a LifecycleCache, title: &str) -> &'a CachedEntity {
    cache
        .entities
        .values()
        .find(|e| e.fingerprint.title == title)
        .unwrap_or_else(|| panic!("no cached entity found with title {title:?}"))
}

fn milestone_for_name<'a>(cache: &'a LifecycleCache, name: &str) -> &'a CachedMilestone {
    cache
        .milestones
        .values()
        .find(|m| m.name == name)
        .unwrap_or_else(|| panic!("no cached milestone found with name {name:?}"))
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

#[test]
fn records_done_transition_with_commit_hash_and_date() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "close", "2026-07-11T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    let entity = entity_for_title(&cache, "task a");
    assert_eq!(entity.last_known_status, CachedStatus::Done);
    let done = entity
        .transitions
        .iter()
        .find(|t| matches!(t.kind, TransitionKind::Done))
        .expect("expected a Done transition");
    assert_eq!(done.date, "2026-07-11");
    assert_eq!(done.commit_hash, cache.head_commit);
}

#[test]
fn records_cancelled_transition() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [-] task a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "cancel", "2026-07-11T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    let entity = entity_for_title(&cache, "task a");
    assert_eq!(entity.last_known_status, CachedStatus::Cancelled);
    assert!(
        entity
            .transitions
            .iter()
            .any(|t| matches!(t.kind, TransitionKind::Cancelled)),
        "transitions: {:?}",
        entity.transitions
    );
}

#[test]
fn records_reopened_transition_when_done_task_goes_back_to_todo() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "close", "2026-07-11T12:00:00Z");

    let file_content = "\
- [ ] task a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "reopen", "2026-07-12T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    let entity = entity_for_title(&cache, "task a");
    assert_eq!(entity.last_known_status, CachedStatus::Todo);
    let reopened = entity
        .transitions
        .iter()
        .find(|t| matches!(t.kind, TransitionKind::Reopened))
        .expect("expected a Reopened transition");
    assert_eq!(reopened.date, "2026-07-12");
}

#[test]
fn records_deleted_transition_when_task_removed_from_list() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task a
- [ ] task b
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [ ] task b
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "delete a", "2026-07-11T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    let entity = entity_for_title(&cache, "task a");
    assert!(
        entity
            .transitions
            .iter()
            .any(|t| matches!(t.kind, TransitionKind::Deleted)),
        "transitions: {:?}",
        entity.transitions
    );
    // The deleted task no longer appears among live entities for the file.
    let file_state = cache.files.get("tasks.agile.md").expect("file state");
    assert!(
        !file_state.entities.iter().any(|n| n.title == "task a"),
        "deleted task should not be live: {:?}",
        file_state.entities
    );
    assert!(
        file_state.graveyard.iter().any(|n| n.title == "task a"),
        "deleted task should be kept in the graveyard: {:?}",
        file_state.graveyard
    );
}

#[test]
fn deleted_then_recreated_task_reuses_the_same_entity_id() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "delete", "2026-07-11T12:00:00Z");

    let file_content = "\
- [ ] task a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "recreate", "2026-07-12T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    let matches: Vec<&CachedEntity> = cache
        .entities
        .values()
        .filter(|e| e.fingerprint.title == "task a")
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "expected exactly one cached entity for the recreated task, found {}",
        matches.len()
    );
    let entity = matches[0];
    assert!(
        entity
            .transitions
            .iter()
            .any(|t| matches!(t.kind, TransitionKind::Deleted)),
        "transitions: {:?}",
        entity.transitions
    );
}

#[test]
fn records_rank_change_for_top_level_task_only() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task a
  - [ ] child
- [ ] task b
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [ ] task b
- [ ] task a
  - [ ] child
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "reorder", "2026-07-11T12:00:00Z");

    let cache = update(dir.path()).expect("cache");

    let task_a = entity_for_title(&cache, "task a");
    assert_eq!(task_a.last_known_rank, Some(2));
    assert!(
        task_a.transitions.iter().any(|t| matches!(
            t.kind,
            TransitionKind::RankChanged {
                old_rank: 1,
                new_rank: 2
            }
        )),
        "transitions: {:?}",
        task_a.transitions
    );

    let child = entity_for_title(&cache, "child");
    assert_eq!(child.last_known_rank, None, "subtasks should have no rank");
    assert!(
        !child
            .transitions
            .iter()
            .any(|t| matches!(t.kind, TransitionKind::RankChanged { .. })),
        "subtasks should never record rank changes: {:?}",
        child.transitions
    );
}

#[test]
fn records_milestone_rank_change_when_preceding_task_rank_shifts() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
- [ ] task b
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [ ] task zero
- [ ] task a
#MILESTONE: alpha
- [ ] task b
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "insert task before", "2026-07-11T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    let milestone = milestone_for_name(&cache, "alpha");
    assert_eq!(milestone.last_known_rank, Some(2));
    assert!(
        milestone.transitions.iter().any(|t| matches!(
            t.kind,
            MilestoneTransitionKind::RankChanged {
                old_rank: Some(1),
                new_rank: Some(2)
            }
        )),
        "transitions: {:?}",
        milestone.transitions
    );
}

#[test]
fn records_milestone_deleted_transition() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
- [ ] task b
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [ ] task a
- [ ] task b
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "remove milestone", "2026-07-11T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    let milestone = milestone_for_name(&cache, "alpha");
    assert!(
        milestone
            .transitions
            .iter()
            .any(|t| matches!(t.kind, MilestoneTransitionKind::Deleted)),
        "transitions: {:?}",
        milestone.transitions
    );
}

#[test]
fn todo_done_timeline_tracks_completion_over_commits_scoped_to_milestone() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task a
  - [ ] sub a1
- [ ] task b
#MILESTONE: alpha
- [ ] task c
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [x] task a
  - [x] sub a1
- [ ] task b
#MILESTONE: alpha
- [ ] task c
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "finish task a", "2026-07-11T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    let milestone = milestone_for_name(&cache, "alpha");
    let target_rank = milestone.last_known_rank;
    assert_eq!(target_rank, Some(2), "milestone should rank after task b");

    let mut commits = git::commits(dir.path());
    commits.reverse();
    let points = todo_done_timeline(&cache, &commits, target_rank);

    assert_eq!(points.len(), 2, "expected one point per commit: {points:?}");
    // task c is after the milestone, so only task a (+ its subtask) and task b
    // are ever in scope: total weight = 1.0 (task a) + 0.5 (sub a1) + 1.0 (task b) = 2.5.
    assert_eq!(points[0].total_weight, 2.5);
    assert_eq!(points[0].total_count, 3);
    assert_eq!(points[0].done_weight, 0.0);
    assert_eq!(points[0].done_count, 0);

    assert_eq!(points[1].total_weight, 2.5);
    assert_eq!(points[1].total_count, 3);
    assert_eq!(points[1].done_weight, 1.5, "task a + sub a1 now done");
    assert_eq!(points[1].done_count, 2);
}

#[test]
fn todo_done_timeline_excludes_deleted_entities_from_that_commit_onward() {
    let dir = setup_repo();
    let file_content = "\
- [ ] task a
- [ ] task b
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "initial", "2026-07-10T12:00:00Z");

    let file_content = "\
- [ ] task a
";
    std::fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    commit_all_at(dir.path(), "delete task b", "2026-07-11T12:00:00Z");

    let cache = update(dir.path()).expect("cache");
    let mut commits = git::commits(dir.path());
    commits.reverse();
    // Rank 2 so both task a and task b would be in scope if not deleted.
    let points = todo_done_timeline(&cache, &commits, Some(2));

    assert_eq!(points[0].total_count, 2, "both tasks present initially");
    assert_eq!(
        points[1].total_count, 1,
        "deleted task should no longer count"
    );
}
