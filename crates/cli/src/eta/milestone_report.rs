//! `agile milestones`' list and detail text reports: git-independent
//! current-state formatting on top of [`super::milestone_stats`]. No
//! ETA/time estimation lives here — that's `report.rs`/`velocity.rs` (used
//! by `agile when`).

use super::milestone_stats::{self, MilestoneStats};
use std::path::Path;

/// Builds the bare `agile milestones` list: one line per future milestone,
/// in backlog order, ranked starting at 1. Shows "no milestones" if there
/// are none. `by_count` selects top-level task counts (`--count`) instead
/// of the default weighted counts.
pub fn build_milestones_list_report(root: &Path, by_count: bool) -> String {
    let stats = milestone_stats::collect_future_milestone_stats(root);
    if stats.is_empty() {
        return "no milestones\n".to_string();
    }
    stats
        .iter()
        .map(|s| render_milestone_list_line(s, by_count))
        .collect()
}

/// Builds the `agile milestones --next <rank>` detail report: the
/// milestone's name plus both its task-count and weight breakdowns.
/// Errors if no future milestone has that rank.
pub fn build_milestone_detail_report(root: &Path, rank: usize) -> Result<String, String> {
    let stats = milestone_stats::milestone_stats_for_rank(root, rank)
        .ok_or_else(|| format!("milestone rank {rank} does not exist"))?;
    Ok(render_milestone_detail_report(&stats))
}

/// Renders one `agile milestones` list line: rank and name left-padded to
/// line up the done/total counts across milestones, e.g.
/// `1 alpha                 2 / 3 66%`.
fn render_milestone_list_line(stats: &MilestoneStats, by_count: bool) -> String {
    let prefix = format!("{} {}", stats.rank, stats.name);
    let (done, total, pct) = if by_count {
        (
            stats.done_top_level.to_string(),
            stats.total_top_level.to_string(),
            stats.percentage_count(),
        )
    } else {
        (
            format_weight(stats.done_weight),
            format_weight(stats.total_weight),
            stats.percentage_weight(),
        )
    };
    format!("{prefix:<23}{done:>2} / {total} {pct}%\n")
}

/// Renders the `agile milestones --next <rank>` detail block: both the
/// task-count and weight breakdowns since the previous milestone.
fn render_milestone_detail_report(stats: &MilestoneStats) -> String {
    let tasks_todo = stats.total_top_level - stats.done_top_level;
    let weight_todo = stats.total_weight - stats.done_weight;
    format!(
        "milestone: {}\n\
         tasks since last milestone: {}\n\
         tasks to do: {}\n\
         tasks done: {}\n\
         tasks percentage done: {}%\n\
         weight to do: {}\n\
         weight done: {}\n\
         weight percentage done: {}%\n",
        stats.name,
        stats.total_top_level,
        tasks_todo,
        stats.done_top_level,
        stats.percentage_count(),
        format_weight(weight_todo),
        format_weight(stats.done_weight),
        stats.percentage_weight(),
    )
}

/// Formats a weight value rounded to 2 decimal places, trimming trailing
/// zeros (and a trailing `.` if the result is a whole number) so `6.0`
/// prints as `6` rather than `6.00`.
fn format_weight(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    let mut s = format!("{rounded:.2}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

#[cfg(test)]
#[path = "milestone_report_tests.rs"]
mod tests;
