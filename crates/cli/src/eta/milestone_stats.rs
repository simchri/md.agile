//! Current-state milestone stats for `agile milestones`: git-independent
//! done/total counts (both weighted and top-level-task-counted) per
//! *future* milestone span, computed straight from the live worktree. No
//! history/velocity/ETA math lives here — see `report.rs`/`velocity.rs`
//! for that side of milestone reporting.

use super::velocity::weight_for_depth;
use crate::cli::common::find_task_files;
use crate::parser::{self, FileItem, Status, Subtask, Task};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct MilestoneStats {
    /// 1-based rank among *future* milestones only, starting at 1 for the
    /// next milestone to be reached (matching `agile when --next <rank>`'s
    /// rank space).
    pub rank: usize,
    pub name: String,
    /// Weighted total across this milestone's span (since the previous
    /// milestone, or the start of the backlog for the first one):
    /// top-level tasks weigh 1, subtasks at depth `n` weigh `1/n`.
    pub total_weight: f64,
    pub done_weight: f64,
    /// Same span, counting only top-level tasks (no subtasks).
    pub total_top_level: usize,
    pub done_top_level: usize,
}

impl MilestoneStats {
    /// Percentage done, based on weighted counts, floored to the nearest
    /// whole percent so `100%` only ever shows once the span is fully done.
    pub fn percentage_weight(&self) -> u32 {
        floor_percentage(self.done_weight, self.total_weight)
    }

    /// Percentage done, based on top-level task counts (`--count` mode),
    /// with the same flooring rule as [`Self::percentage_weight`].
    pub fn percentage_count(&self) -> u32 {
        floor_percentage(self.done_top_level as f64, self.total_top_level as f64)
    }
}

fn floor_percentage(done: f64, total: f64) -> u32 {
    if total <= 0.0 {
        return 0;
    }
    ((done / total) * 100.0).floor().clamp(0.0, 100.0) as u32
}

#[derive(Default)]
struct SpanAccumulator {
    total_weight: f64,
    done_weight: f64,
    total_top_level: usize,
    done_top_level: usize,
}

impl SpanAccumulator {
    fn add_task(&mut self, task: &Task) {
        self.total_top_level += 1;
        self.total_weight += 1.0;
        if is_closed(&task.status) {
            self.done_top_level += 1;
            self.done_weight += 1.0;
        }
        self.add_subtasks(&task.children, 2);
    }

    fn add_subtasks(&mut self, children: &[Subtask], depth: usize) {
        for child in children {
            let weight = weight_for_depth(depth);
            self.total_weight += weight;
            if is_closed(&child.status) {
                self.done_weight += weight;
            }
            self.add_subtasks(&child.children, depth + 1);
        }
    }

    fn finish(self, rank: usize, name: String) -> MilestoneStats {
        MilestoneStats {
            rank,
            name,
            total_weight: self.total_weight,
            done_weight: self.done_weight,
            total_top_level: self.total_top_level,
            done_top_level: self.done_top_level,
        }
    }
}

fn is_closed(status: &Status) -> bool {
    matches!(status, Status::Done | Status::Cancelled)
}

fn task_subtree_complete(task: &Task) -> bool {
    is_closed(&task.status) && task.children.iter().all(subtask_subtree_complete)
}

fn subtask_subtree_complete(subtask: &Subtask) -> bool {
    is_closed(&subtask.status) && subtask.children.iter().all(subtask_subtree_complete)
}

/// Returns stats for every *future* milestone (those after the first
/// incomplete top-level task in the backlog), in backlog order, ranked
/// starting at 1 for the next milestone to be reached. Each milestone's
/// stats cover only its own span (since the previous milestone, or the
/// start of the backlog for the first one) — always accumulated in full
/// regardless of when the span turned out to contain the first incomplete
/// task, so already-done tasks earlier in a future milestone's own span
/// still count toward its totals.
pub fn collect_future_milestone_stats(root: &Path) -> Vec<MilestoneStats> {
    let mut seen_incomplete_task = false;
    let mut current = SpanAccumulator::default();
    let mut stats = Vec::new();

    for path in find_task_files(root) {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let items = parser::parse(&content, path);
        for item in items {
            match item {
                FileItem::Task(task) => {
                    if !seen_incomplete_task && !task_subtree_complete(&task) {
                        seen_incomplete_task = true;
                    }
                    current.add_task(&task);
                }
                FileItem::Milestone(m) => {
                    if seen_incomplete_task {
                        let rank = stats.len() + 1;
                        stats.push(current.finish(rank, m.name));
                    }
                    current = SpanAccumulator::default();
                }
            }
        }
    }
    stats
}

/// Returns the future-milestone stats for the given 1-based rank, or
/// `None` if no future milestone has that rank (matching
/// [`collect_future_milestone_stats`]'s rank space).
pub fn milestone_stats_for_rank(root: &Path, rank: usize) -> Option<MilestoneStats> {
    let index = rank.checked_sub(1)?;
    collect_future_milestone_stats(root).into_iter().nth(index)
}

#[cfg(test)]
#[path = "milestone_stats_tests.rs"]
mod tests;
