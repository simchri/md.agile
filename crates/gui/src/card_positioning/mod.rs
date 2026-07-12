use crate::server::{TaskStatus, TaskView};

// --- Diagonal-track geometry -----------------------------------------------
//
// Single source of truth for the in-progress card layout. Both
// [`in_progress_style`] (CSS `calc()` consumed by Dioxus) and
// [`card_top_left_px`] (pixel coords consumed by the physics integrator's
// boundary checks) derive from these constants, so a change here flows
// through to both.

/// Card edge length on the canvas (cards are square).
pub const CARD_PX: f64 = 220.0;
/// Visual gap between the card edge and the start/end of the diagonal track.
pub const EDGE_MARGIN_PX: f64 = 5.0;
/// Total px the diagonal track is inset by — the card itself plus a margin
/// on each end. Used by both the CSS `calc(...)` formula and the pixel
/// boundary calc.
pub const TRACK_INSET_PX: f64 = CARD_PX + 2.0 * EDGE_MARGIN_PX;
/// Vertical fraction of the viewport at which the diagonal starts (just
/// below the top separator).
pub const DIAG_TOP_FRAC: f64 = 0.15;
/// Vertical fraction of the viewport spanned by the diagonal.
pub const DIAG_HEIGHT_FRAC: f64 = 0.70;

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

/// Builds a CSS positioning rule for an in-progress card at the given relative position.
///
/// Input:
/// - `x`: normalized x coordinate (0.0 = left, 1.0 = right)
/// - `y`: normalized y coordinate (0.0 = top, 1.0 = bottom)
///
/// Returns CSS positioning (top/left) in a mix of viewport units and pixels.
pub fn in_progress_style(x: f64, y: f64) -> String {
    let x = x.clamp(0.0, 1.0);
    let y = y.clamp(0.0, 1.0);
    let top_vh = DIAG_TOP_FRAC * 100.0;
    let height_vh = DIAG_HEIGHT_FRAC * 100.0;
    format!(
        "top: calc({top_vh:.0}vh + {EDGE_MARGIN_PX:.0}px + {y:.3} * ({height_vh:.0}vh - {TRACK_INSET_PX:.0}px)); \
         left: calc({EDGE_MARGIN_PX:.0}px + {x:.3} * (100vw - {TRACK_INSET_PX:.0}px));"
    )
}

/// Converts a normalized in-progress card position into the pixel
/// coordinates of its top-left corner, given the current viewport size in
/// pixels. This is the same mapping [`in_progress_style`] expresses as a
/// CSS `calc()` string, expressed instead as plain arithmetic — used to
/// convert a card's physics position into pixels for drag-and-drop.
pub fn card_top_left_px(x: f64, y: f64, viewport_w_px: f64, viewport_h_px: f64) -> (f64, f64) {
    let x = x.clamp(0.0, 1.0);
    let y = y.clamp(0.0, 1.0);
    let left_px = EDGE_MARGIN_PX + x * (viewport_w_px - TRACK_INSET_PX);
    let top_px = DIAG_TOP_FRAC * viewport_h_px
        + EDGE_MARGIN_PX
        + y * (DIAG_HEIGHT_FRAC * viewport_h_px - TRACK_INSET_PX);
    (left_px, top_px)
}

/// Converts pixel coordinates of a card's top-left corner back into the
/// normalized physics coordinate space — the inverse of
/// [`card_top_left_px`]. Used to convert a live mouse position into a
/// physics position while dragging a card. Clamped to `0.0..=1.0` on each
/// axis, since a card can't be dragged off the diagonal track.
pub fn card_position_from_px(
    left_px: f64,
    top_px: f64,
    viewport_w_px: f64,
    viewport_h_px: f64,
) -> (f64, f64) {
    let x = (left_px - EDGE_MARGIN_PX) / (viewport_w_px - TRACK_INSET_PX);
    let y = (top_px - DIAG_TOP_FRAC * viewport_h_px - EDGE_MARGIN_PX)
        / (DIAG_HEIGHT_FRAC * viewport_h_px - TRACK_INSET_PX);
    (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))
}

/// True when the task has at least one direct subtask marked Done or
/// Cancelled — i.e. work has begun on it.
#[cfg(feature = "server")]
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
