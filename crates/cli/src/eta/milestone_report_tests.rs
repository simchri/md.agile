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

    assert_eq!(out, "1 alpha 2 / 3 66%\n");
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

    assert_eq!(weighted, "1 alpha 1 / 2.5 40%\n");
    assert_eq!(counted, "1 alpha 1 / 2 50%\n");
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

    assert_eq!(out, "1 alpha 0 / 1 0%\n2 beta  0 / 1 0%\n");
}

#[test]
fn list_report_pads_names_and_done_counts_to_the_widest_column() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: a
- [x] task b1
- [x] task b2
- [x] task b3
- [x] task b4
- [x] task b5
- [x] task b6
- [x] task b7
- [x] task b8
- [x] task b9
- [x] task b10
- [ ] task b11
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = build_milestones_list_report(dir.path(), true);

    // Name column widens to fit "beta" (4 chars, wider than "a"), the done
    // and total columns widen to fit "10"/"11" (2 digits) so " / " stays
    // aligned, and the percentage column widens to fit "90" (2 digits).
    let expected = "\
1 a     0 /  1  0%
2 beta 10 / 11 90%
";
    assert_eq!(out, expected);
}

#[test]
fn list_report_pads_percentage_column_to_the_widest_value() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
- [x] task b
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = build_milestones_list_report(dir.path(), false);

    // alpha: 0% (1 digit), beta: 100% (3 digits) -> percentage column
    // widens to 3 so both rows' '%' line up.
    let expected = "\
1 alpha 0 / 1   0%
2 beta  1 / 1 100%
";
    assert_eq!(out, expected);
}

#[test]
fn list_report_truncates_long_milestone_names_with_ellipsis() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: this milestone name is way too long to display in full
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = build_milestones_list_report(dir.path(), false);

    assert_eq!(out, "1 this milestone name is way too long to … 0 / 1 0%\n");
}

#[test]
fn truncate_name_leaves_short_names_untouched() {
    assert_eq!(truncate_name("alpha", 20), "alpha");
    assert_eq!(truncate_name(&"a".repeat(20), 20), "a".repeat(20));
}

#[test]
fn truncate_name_shortens_long_names_with_ellipsis() {
    let name = "a".repeat(25);
    let truncated = truncate_name(&name, 20);

    assert_eq!(truncated, format!("{}…", "a".repeat(19)));
    assert_eq!(truncated.chars().count(), 20);
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
fn format_weight_strips_trailing_zero_but_keeps_one_decimal_otherwise() {
    assert_eq!(format_weight(6.0), "6");
    assert_eq!(format_weight(0.0), "0");
    assert_eq!(format_weight(23.333_333), "23.3");
    assert_eq!(format_weight(1.5), "1.5");
}
