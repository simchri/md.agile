use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn list_report_shows_no_milestones_message_when_none_are_future() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = build_milestones_list_report(dir.path(), false);

    assert_eq!(out, "no milestones\n");
}

#[test]
fn list_report_shows_weighted_counts_and_floored_percentage_by_default() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task a
- [x] task b
- [ ] task c
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = build_milestones_list_report(dir.path(), false);

    assert_eq!(out, "1 alpha                 2 / 3 66%\n");
}

#[test]
fn list_report_shows_top_level_task_counts_with_count_flag() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task a
  - [ ] subtask a1
- [ ] task b
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Weighted: done=1.0 (task a; its incomplete subtask contributes 0 done),
    // total=1.0 + 0.5 + 1.0 = 2.5 -> would differ from the plain task count.
    let weighted = build_milestones_list_report(dir.path(), false);
    let counted = build_milestones_list_report(dir.path(), true);

    assert_eq!(weighted, "1 alpha                 1 / 2.5 40%\n");
    assert_eq!(counted, "1 alpha                 1 / 2 50%\n");
}

#[test]
fn list_report_includes_every_future_milestone_in_backlog_order() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
- [ ] task b
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = build_milestones_list_report(dir.path(), false);

    assert_eq!(
        out,
        "1 alpha                 0 / 1 0%\n2 beta                  0 / 1 0%\n"
    );
}

#[test]
fn detail_report_shows_tasks_and_weight_breakdown() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task a
- [x] task b
- [x] task c
- [ ] task d
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = build_milestone_detail_report(dir.path(), 1).unwrap();

    assert_eq!(
        out,
        "\
milestone: alpha
tasks since last milestone: 4
tasks to do: 1
tasks done: 3
tasks percentage done: 75%
weight to do: 1
weight done: 3
weight percentage done: 75%
"
    );
}

#[test]
fn detail_report_errors_for_out_of_range_rank() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let err = build_milestone_detail_report(dir.path(), 5).unwrap_err();

    assert_eq!(err, "milestone rank 5 does not exist");
}

#[test]
fn format_weight_strips_trailing_zeros_but_keeps_two_decimals_otherwise() {
    assert_eq!(format_weight(6.0), "6");
    assert_eq!(format_weight(0.0), "0");
    assert_eq!(format_weight(23.333_333), "23.33");
    assert_eq!(format_weight(1.5), "1.5");
}
