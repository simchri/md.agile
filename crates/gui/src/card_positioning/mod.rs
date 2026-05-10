use crate::server::{TaskStatus, TaskView};

// --- Diagonal-track geometry -----------------------------------------------
//
// Single source of truth for the in-progress card layout. Both
// [`diagonal_style`] (CSS `calc()` consumed by Dioxus) and
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
/// 45° approximation of the perpendicular to the diagonal: positive
/// `perp_offset_px` shifts the card upper-right.
pub const PERP_AXIS: f64 = 0.707;

/// Reference viewport used by the GUI when no real measurement is available.
pub const REFERENCE_VIEWPORT: Viewport = Viewport {
    width_px: 1440.0,
    height_px: 900.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width_px: f64,
    pub height_px: f64,
}

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
/// Sister function to [`card_top_left_px`]: both derive from the same
/// geometry constants ([`EDGE_MARGIN_PX`], [`TRACK_INSET_PX`],
/// [`DIAG_TOP_FRAC`], [`DIAG_HEIGHT_FRAC`], [`PERP_AXIS`]), so the CSS
/// position and the pixel position used by the physics integrator's
/// boundary checks always agree.
pub fn diagonal_style(progress: f64, perp_offset_px: f64) -> String {
    let p = progress.clamp(0.0, 1.0);
    let top_adj = -perp_offset_px * PERP_AXIS;
    let left_adj = perp_offset_px * PERP_AXIS;
    let top_vh = DIAG_TOP_FRAC * 100.0;
    let height_vh = DIAG_HEIGHT_FRAC * 100.0;
    format!(
        "top: calc({top_vh:.0}vh + {EDGE_MARGIN_PX:.0}px + {p:.3} * ({height_vh:.0}vh - {TRACK_INSET_PX:.0}px) + {top_adj:.1}px); \
         left: calc({EDGE_MARGIN_PX:.0}px + {p:.3} * (100vw - {TRACK_INSET_PX:.0}px) + {left_adj:.1}px);"
    )
}

/// Top-left pixel coordinates of an in-progress card. Mirrors the formula in
/// [`diagonal_style`] so the physics integrator's boundary checks line up
/// with where CSS actually places the card.
pub fn card_top_left_px(progress: f64, perp_offset_px: f64, viewport: Viewport) -> (f64, f64) {
    let left = EDGE_MARGIN_PX
        + progress * (viewport.width_px - TRACK_INSET_PX)
        + perp_offset_px * PERP_AXIS;
    let top = DIAG_TOP_FRAC * viewport.height_px
        + EDGE_MARGIN_PX
        + progress * (DIAG_HEIGHT_FRAC * viewport.height_px - TRACK_INSET_PX)
        - perp_offset_px * PERP_AXIS;
    (left, top)
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
