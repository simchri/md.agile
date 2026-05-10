use crate::server::{TaskStatus, TaskView};

/// Returns the completion ratio (0.0..=1.0) used to position the post-it on
/// the diagonal. The top-level checkbox is worth a flat 10% of the total —
/// reserved for the moment the user actually ticks the parent task done — so
/// even a Todo task with every subtask complete tops out at 0.9. Subtasks
/// (counted recursively, with Done and Cancelled treated as complete) fill
/// the remaining 90% proportionally.
pub fn task_progress(task: &TaskView) -> f64 {
    const PARENT_WEIGHT: f64 = 0.1;
    const SUBTASKS_WEIGHT: f64 = 1.0 - PARENT_WEIGHT;

    if matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled) {
        return 1.0;
    }
    let (done, total) = count_subtasks(task);
    let subtasks_share = if total == 0 {
        0.0
    } else {
        done as f64 / total as f64
    };
    SUBTASKS_WEIGHT * subtasks_share
}

pub fn count_subtasks(task: &TaskView) -> (usize, usize) {
    let mut done = 0;
    let mut total = 0;
    for child in &task.children {
        total += 1;
        if matches!(child.status, TaskStatus::Done | TaskStatus::Cancelled) {
            done += 1;
        }
        let (cd, ct) = count_subtasks(child);
        done += cd;
        total += ct;
    }
    (done, total)
}

/// Builds a CSS positioning rule that places the post-it along the top-left to
/// bottom-right diagonal at `progress` (0.0 = top-left, 1.0 = bottom-right),
/// then shifts it `perp_offset_px` pixels perpendicular to the diagonal
/// (positive = upper-right, negative = lower-left).
///
/// Vertical range: separator1 (15vh) to separator2 (85vh), leaving a 5px gap
/// at each end and room for the 220px card height (range = 70vh - 230px).
/// Horizontal range: 5px from each edge, with 220px card width (range = 100vw - 230px).
pub fn diagonal_style(progress: f64, perp_offset_px: f64) -> String {
    let p = progress.clamp(0.0, 1.0);
    // 45° approximation of the perpendicular to the diagonal: (+left, -top)
    let top_adj = -perp_offset_px * 0.707;
    let left_adj = perp_offset_px * 0.707;
    format!(
        "top: calc(15vh + 5px + {p:.3} * (70vh - 230px) + {top_adj:.1}px); left: calc(5px + {p:.3} * (100vw - 230px) + {left_adj:.1}px);"
    )
}

/// True when the task has at least one direct subtask marked Done or
/// Cancelled — i.e. work has begun on it.
pub fn has_started(task: &TaskView) -> bool {
    task.children
        .iter()
        .any(|c| matches!(c.status, TaskStatus::Done | TaskStatus::Cancelled))
}

/// Backlog cards are laid out left-to-right at the top of the canvas. The
/// lowest-ranked task on screen anchors x = `CARD_GAP_PX`; subsequent ranks
/// step right by one card width plus a gap.
pub fn backlog_position_style(task_rank: usize, lowest_rank_on_screen: usize) -> String {
    let pos_index = task_rank.saturating_sub(lowest_rank_on_screen);
    let x_px = CARD_GAP_PX + pos_index * (CARD_WIDTH_PX + CARD_GAP_PX);
    format!("top: 5px; bottom: unset; left: {x_px}px;")
}

/// Done cards are laid out right-to-left at the bottom of the canvas. The
/// highest-ranked done task on screen sits flush against the right edge;
/// lower ranks step left.
pub fn done_position_style(task_rank: usize, highest_rank_on_screen: usize) -> String {
    let pos_index = highest_rank_on_screen.saturating_sub(task_rank);
    let x_px = CARD_GAP_PX + (pos_index + 1) * (CARD_WIDTH_PX + CARD_GAP_PX);
    format!("top: calc(85vh + 5px); left: calc(100vw - {x_px}px);")
}

const CARD_WIDTH_PX: usize = 110;
const CARD_GAP_PX: usize = 8;

pub fn status_box(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "[ ]",
        TaskStatus::Done => "[x]",
        TaskStatus::Cancelled => "[-]",
    }
}

pub fn status_class(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "status-todo",
        TaskStatus::Done => "status-done",
        TaskStatus::Cancelled => "status-cancelled",
    }
}

#[cfg(test)]
mod tests;
