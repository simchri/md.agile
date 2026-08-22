//! `agile milestones`' list and detail text reports: git-independent
//! current-state formatting on top of [`super::milestone_stats`]. No
//! ETA/time estimation lives here — that's `report.rs`/`velocity.rs` (used
//! by `agile when`).

use super::milestone_stats::{self, MilestoneStats};
use std::path::Path;

/// Milestone names are truncated to this many characters (visible, i.e. not
/// counting the trailing `…`) in the `agile milestones` list, so a single
/// long name can't throw off the alignment of every other row.
const MAX_NAME_LEN: usize = 40;

/// Builds the bare `agile milestones` list: one line per future milestone,
/// in backlog order, ranked starting at 1. Shows "no milestones" if there
/// are none. `by_count` selects top-level task counts (`--count`) instead
/// of the default weighted counts. Columns are padded to line up across all
/// rows (rank, name, done count, and percentage), and long names are
/// shortened with a trailing `…` so they can't break that alignment.
pub fn build_milestones_list_report(root: &Path, by_count: bool) -> String {
    let stats = milestone_stats::collect_future_milestone_stats(root);
    if stats.is_empty() {
        return "no milestones\n".to_string();
    }
    render_milestone_list_lines(&stats, by_count)
}

/// Builds the `agile milestones --next <rank>` detail report: the
/// milestone's name plus both its task-count and weight breakdowns.
/// Errors if no future milestone has that rank.
pub fn build_milestone_detail_report(root: &Path, rank: usize) -> Result<String, String> {
    let stats = milestone_stats::milestone_stats_for_rank(root, rank)
        .ok_or_else(|| format!("milestone rank {rank} does not exist"))?;
    Ok(render_milestone_detail_report(&stats))
}

/// Renders every `agile milestones` list line, with rank, (possibly
/// shortened) name, done count, total, and percentage padded to line up
/// across all rows — based on the widest value in each column for this
/// particular report, not a fixed width, so short lists stay compact.
fn render_milestone_list_lines(stats: &[MilestoneStats], by_count: bool) -> String {
    let rank_width = stats
        .iter()
        .map(|s| s.rank.to_string().len())
        .max()
        .unwrap_or(1);
    let names: Vec<String> = stats
        .iter()
        .map(|s| truncate_name(&s.name, MAX_NAME_LEN))
        .collect();
    let name_width = names.iter().map(|n| n.chars().count()).max().unwrap_or(0);
    let counts: Vec<(String, String, u32)> = stats
        .iter()
        .map(|s| {
            if by_count {
                (
                    s.done_top_level.to_string(),
                    s.total_top_level.to_string(),
                    s.percentage_count(),
                )
            } else {
                (
                    format_weight(s.done_weight),
                    format_weight(s.total_weight),
                    s.percentage_weight(),
                )
            }
        })
        .collect();
    let done_width = counts
        .iter()
        .map(|(done, ..)| done.len())
        .max()
        .unwrap_or(0);
    let total_width = counts
        .iter()
        .map(|(_, total, _)| total.len())
        .max()
        .unwrap_or(0);
    let pct_width = counts
        .iter()
        .map(|(.., pct)| pct.to_string().len())
        .max()
        .unwrap_or(0);

    stats
        .iter()
        .zip(&names)
        .zip(&counts)
        .map(|((s, name), (done, total, pct))| {
            let rank = s.rank;
            format!(
                "{rank:>rank_width$} {name:<name_width$} {done:>done_width$} / {total:>total_width$} {pct:>pct_width$}%\n"
            )
        })
        .collect()
}

/// Shortens `name` to at most `max_len` characters (counting the trailing
/// `…` itself), leaving it untouched if it's already short enough, so a
/// single long milestone name can't push the rest of a row's columns out of
/// alignment.
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.chars().count() <= max_len {
        return name.to_string();
    }
    let truncated: String = name.chars().take(max_len.saturating_sub(1)).collect();
    format!("{truncated}…")
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

/// Formats a weight value rounded to 1 decimal place, trimming a trailing
/// `0` (and the `.` itself if the result is a whole number) so `6.0`
/// prints as `6` rather than `6.0`.
fn format_weight(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    let mut s = format!("{rounded:.1}");
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
