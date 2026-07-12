use super::*;
use crate::server::{TaskStatus, TaskView};

fn todo_task(title: &str, children: Vec<TaskView>) -> TaskView {
    TaskView {
        status: TaskStatus::Todo,
        title: title.to_string(),
        markers: vec![],
        body: vec![],
        children,
        rank: 0,
        path: String::new(),
        line: 0,
    }
}

fn done_task(title: &str) -> TaskView {
    TaskView {
        status: TaskStatus::Done,
        title: title.to_string(),
        markers: vec![],
        body: vec![],
        children: vec![],
        rank: 0,
        path: String::new(),
        line: 0,
    }
}

fn cancelled_task(title: &str) -> TaskView {
    TaskView {
        status: TaskStatus::Cancelled,
        title: title.to_string(),
        markers: vec![],
        body: vec![],
        children: vec![],
        rank: 0,
        path: String::new(),
        line: 0,
    }
}

// --- task_progress ---

#[test]
fn progress_done_parent_is_1() {
    let task = TaskView {
        status: TaskStatus::Done,
        ..todo_task("t", vec![])
    };
    assert_eq!(task_progress(&task), 1.0);
}

#[test]
fn progress_cancelled_parent_is_1() {
    let task = TaskView {
        status: TaskStatus::Cancelled,
        ..todo_task("t", vec![])
    };
    assert_eq!(task_progress(&task), 1.0);
}

#[test]
fn progress_todo_no_children_is_0() {
    let task = todo_task("t", vec![]);
    assert_eq!(task_progress(&task), 0.0);
}

#[test]
fn progress_all_subtasks_done_parent_todo_is_0_9() {
    let task = todo_task("t", vec![done_task("a"), done_task("b")]);
    assert!((task_progress(&task) - 0.9).abs() < 1e-9);
}

#[test]
fn progress_half_subtasks_done() {
    let task = todo_task("t", vec![done_task("a"), todo_task("b", vec![])]);
    // 0.9 * (1/2) = 0.45
    assert!((task_progress(&task) - 0.45).abs() < 1e-9);
}

#[test]
fn progress_cancelled_subtask_counts_as_done() {
    let task = todo_task("t", vec![cancelled_task("a"), todo_task("b", vec![])]);
    assert!((task_progress(&task) - 0.45).abs() < 1e-9);
}

// --- count_subtasks ---

#[test]
fn count_no_children() {
    assert_eq!(count_subtasks(&todo_task("t", vec![])), (0, 0));
}

#[test]
fn count_flat_children() {
    let task = todo_task("t", vec![done_task("a"), todo_task("b", vec![])]);
    assert_eq!(count_subtasks(&task), (1, 2));
}

#[test]
fn count_nested_children_recursive() {
    // t
    //   a [done]
    //   b [todo]
    //     c [done]
    let task = todo_task(
        "t",
        vec![done_task("a"), todo_task("b", vec![done_task("c")])],
    );
    // total = a + b + c = 3, done = a + c = 2
    assert_eq!(count_subtasks(&task), (2, 3));
}

// --- in_progress_style (CSS positioning from physics output) ---

#[test]
fn in_progress_style_at_origin_top_left() {
    let s = in_progress_style(0.0, 0.0);
    assert!(s.contains("0.000"), "expected 0.000 in: {s}");
}

#[test]
fn in_progress_style_at_bottom_right() {
    let s = in_progress_style(1.0, 1.0);
    assert!(s.contains("1.000"), "expected 1.000 in: {s}");
}

#[test]
fn in_progress_style_clamps_below_zero() {
    let s = in_progress_style(-5.0, -5.0);
    assert!(s.contains("0.000"), "expected clamp to 0 in: {s}");
}

#[test]
fn in_progress_style_clamps_above_one() {
    let s = in_progress_style(2.0, 2.0);
    assert!(s.contains("1.000"), "expected clamp to 1 in: {s}");
}

// --- has_started ---

#[cfg(feature = "server")]
#[test]
fn has_started_no_children_false() {
    assert!(!has_started(&todo_task("t", vec![])));
}

#[cfg(feature = "server")]
#[test]
fn has_started_only_todo_children_false() {
    let task = todo_task("t", vec![todo_task("a", vec![]), todo_task("b", vec![])]);
    assert!(!has_started(&task));
}

#[cfg(feature = "server")]
#[test]
fn has_started_one_done_child_true() {
    let task = todo_task("t", vec![todo_task("a", vec![]), done_task("b")]);
    assert!(has_started(&task));
}

#[cfg(feature = "server")]
#[test]
fn has_started_cancelled_child_counts() {
    let task = todo_task("t", vec![cancelled_task("a")]);
    assert!(has_started(&task));
}

// --- status_box ---

#[test]
fn status_box_values() {
    assert_eq!(status_box(&TaskStatus::Todo), "[ ]");
    assert_eq!(status_box(&TaskStatus::Done), "[x]");
    assert_eq!(status_box(&TaskStatus::Cancelled), "[-]");
}

// --- status_class ---

#[test]
fn status_class_values() {
    assert_eq!(status_class(&TaskStatus::Todo), "status-todo");
    assert_eq!(status_class(&TaskStatus::Done), "status-done");
    assert_eq!(status_class(&TaskStatus::Cancelled), "status-cancelled");
}

// --- backlog_position_style ---

#[test]
fn backlog_lowest_rank_anchors_at_left_gap() {
    // Task at the lowest rank on screen → pos_index = 0 → left = 8 (CARD_GAP_PX).
    let s = backlog_position_style(5, 5);
    assert!(s.contains("left: 8px"), "got: {s}");
    assert!(s.contains("top: 5px"), "got: {s}");
}

#[test]
fn backlog_steps_right_by_card_plus_gap() {
    // Second-lowest rank → pos_index = 1 → left = 8 + 1*(110+8) = 126.
    let s = backlog_position_style(6, 5);
    assert!(s.contains("left: 126px"), "got: {s}");
}

#[test]
fn backlog_rank_below_lowest_clamps_to_left_anchor() {
    // Defensive: a rank lower than the on-screen minimum shouldn't underflow.
    let s = backlog_position_style(2, 5);
    assert!(s.contains("left: 8px"), "got: {s}");
}

// --- done_position_style ---

#[test]
fn done_highest_rank_anchors_one_card_from_right() {
    // Highest-ranked done task on screen → pos_index = 0 → x_px = 8 + 1*(110+8) = 126.
    let s = done_position_style(10, 10);
    assert!(s.contains("left: calc(100vw - 126px)"), "got: {s}");
    assert!(s.contains("top: calc(85vh + 5px)"), "got: {s}");
}

#[test]
fn done_steps_left_for_lower_ranks() {
    // pos_index = 1 → x_px = 8 + 2*(110+8) = 244.
    let s = done_position_style(9, 10);
    assert!(s.contains("left: calc(100vw - 244px)"), "got: {s}");
}

#[test]
fn done_rank_above_highest_clamps_to_right_anchor() {
    // Defensive: a rank above the on-screen max shouldn't underflow.
    let s = done_position_style(15, 10);
    assert!(s.contains("left: calc(100vw - 126px)"), "got: {s}");
}

// --- in_progress_style ---
//
// These tests anchor the constants embedded in the CSS string. They will
// fail if anyone changes EDGE_MARGIN_PX, TRACK_INSET_PX, DIAG_TOP_FRAC,
// or DIAG_HEIGHT_FRAC, forcing the tests to be updated alongside the
// constant — which is exactly what we want.

#[test]
fn in_progress_style_at_origin_anchors_top_left() {
    let s = in_progress_style(0.0, 0.0);
    assert!(s.contains("top: calc(15vh + 5px"), "got: {s}");
    assert!(s.contains("left: calc(5px"), "got: {s}");
}

#[test]
fn in_progress_style_uses_track_inset_constant() {
    let s = in_progress_style(0.5, 0.5);
    // Both axes use the same TRACK_INSET_PX (= 230 by default).
    assert!(s.contains("(70vh - 230px)"), "got: {s}");
    assert!(s.contains("(100vw - 230px)"), "got: {s}");
}
