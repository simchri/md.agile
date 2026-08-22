use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn collect_future_milestone_stats_skips_already_reached_milestones() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task a
#MILESTONE: alpha
- [ ] task b
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stats = collect_future_milestone_stats(dir.path());

    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].name, "beta");
}

#[test]
fn collect_future_milestone_stats_ranks_from_one_for_next_milestone() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
- [ ] task b
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stats = collect_future_milestone_stats(dir.path());

    assert_eq!(stats[0].rank, 1);
    assert_eq!(stats[0].name, "alpha");
    assert_eq!(stats[1].rank, 2);
    assert_eq!(stats[1].name, "beta");
}

#[test]
fn collect_future_milestone_stats_counts_top_level_tasks_since_previous_milestone() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
- [x] task b
- [x] task c
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stats = collect_future_milestone_stats(dir.path());

    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].total_top_level, 3);
    assert_eq!(stats[0].done_top_level, 2);
    assert_eq!(stats[0].total_weight, 3.0);
    assert_eq!(stats[0].done_weight, 2.0);
}

#[test]
fn collect_future_milestone_stats_weighs_subtasks_by_depth() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task a
  - [x] subtask a1
    - [ ] subtask a1a
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stats = collect_future_milestone_stats(dir.path());

    // task a: weight 1 (done); subtask a1 (depth 2): weight 0.5 (done);
    // subtask a1a (depth 3): weight 1/3 (todo).
    assert_eq!(stats[0].total_weight, 1.0 + 0.5 + 1.0 / 3.0);
    assert_eq!(stats[0].done_weight, 1.0 + 0.5);
    assert_eq!(stats[0].total_top_level, 1);
    assert_eq!(stats[0].done_top_level, 1);
}

#[test]
fn collect_future_milestone_stats_resets_span_after_each_milestone() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
- [ ] task b
- [ ] task c
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stats = collect_future_milestone_stats(dir.path());

    assert_eq!(stats[0].name, "alpha");
    assert_eq!(stats[0].total_top_level, 1);
    assert_eq!(stats[1].name, "beta");
    assert_eq!(stats[1].total_top_level, 2);
}

#[test]
fn milestone_stats_for_rank_returns_none_for_out_of_range_rank() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    assert!(milestone_stats_for_rank(dir.path(), 2).is_none());
    assert!(milestone_stats_for_rank(dir.path(), 0).is_none());
}

#[test]
fn milestone_stats_for_rank_returns_matching_stats() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
- [ ] task b
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stats = milestone_stats_for_rank(dir.path(), 2).unwrap();

    assert_eq!(stats.name, "beta");
}

#[test]
fn percentage_floors_instead_of_rounding() {
    // 3/21 = 14.28..% -> floors to 14%, not 15%.
    let stats = MilestoneStats {
        rank: 1,
        name: "alpha".to_string(),
        total_weight: 21.0,
        done_weight: 3.0,
        total_top_level: 21,
        done_top_level: 3,
    };
    assert_eq!(stats.percentage_weight(), 14);
    assert_eq!(stats.percentage_count(), 14);
}

#[test]
fn percentage_is_100_only_when_fully_done() {
    let almost_done = MilestoneStats {
        rank: 1,
        name: "alpha".to_string(),
        total_weight: 100.0,
        done_weight: 99.9,
        total_top_level: 1000,
        done_top_level: 999,
    };
    assert_eq!(almost_done.percentage_weight(), 99);
    assert_eq!(almost_done.percentage_count(), 99);

    let fully_done = MilestoneStats {
        rank: 1,
        name: "alpha".to_string(),
        total_weight: 100.0,
        done_weight: 100.0,
        total_top_level: 1000,
        done_top_level: 1000,
    };
    assert_eq!(fully_done.percentage_weight(), 100);
    assert_eq!(fully_done.percentage_count(), 100);
}

#[test]
fn percentage_is_zero_when_span_has_no_tasks() {
    let stats = MilestoneStats {
        rank: 1,
        name: "alpha".to_string(),
        total_weight: 0.0,
        done_weight: 0.0,
        total_top_level: 0,
        done_top_level: 0,
    };
    assert_eq!(stats.percentage_weight(), 0);
    assert_eq!(stats.percentage_count(), 0);
}
