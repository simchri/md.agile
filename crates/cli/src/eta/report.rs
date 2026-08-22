//! Text-only reports: the bare `agile when` list (ETA span per future
//! milestone) and `agile when --velocity`'s velocity/creep summary. Neither
//! is a chart, but both build on the same trend/ETA machinery.

use super::eta_math::{EtaEstimate, eta_for_plot};
use super::eta_text::eta_span;
use super::plot_data::build_todo_done_plot;
use super::trend::TrendFitAlgorithm;
use super::velocity::{VelocityEstimate, require_git_repo};
use crate::cli::common::find_task_files;
use crate::parser::{self, FileItem, Status};
use std::path::Path;

/// Builds the bare `agile when` report: the ETA time span for every future
/// milestone (see [`future_milestone_names`]), one per line, in backlog
/// order — matching README.vision.md's list-mode output. A milestone whose
/// ETA can't be computed (e.g. not committed yet, or no convergent trend)
/// shows "unknown" instead of a span.
pub fn build_when_report(root: &Path, algorithm: TrendFitAlgorithm) -> Result<String, String> {
    require_git_repo(root)?;
    let today = super::date_utils::today_unix_days();
    let mut out = String::new();
    for (index, name) in future_milestone_names(root).into_iter().enumerate() {
        let rank = index + 1;
        let eta = build_todo_done_plot(root, rank)
            .ok()
            .and_then(|plot| eta_for_plot(&plot, today, algorithm));
        out.push_str(&render_when_line(&name, eta, today));
    }
    Ok(out)
}

/// Builds the `agile when --next <rank>` detail report: the milestone's
/// name, ETA span, ETA date, and task/weight breakdowns since the
/// previous milestone. Errors if not a git repo or if no future milestone
/// has that rank.
pub fn build_when_detail_report(
    root: &Path,
    rank: usize,
    algorithm: TrendFitAlgorithm,
) -> Result<String, String> {
    require_git_repo(root)?;
    let stats = super::milestone_stats::milestone_stats_for_rank(root, rank)
        .ok_or_else(|| format!("milestone rank {rank} does not exist"))?;

    let today = super::date_utils::today_unix_days();
    let eta = build_todo_done_plot(root, rank)
        .ok()
        .and_then(|plot| eta_for_plot(&plot, today, algorithm));

    let tasks_todo = stats.total_top_level - stats.done_top_level;
    let weight_todo = stats.total_weight - stats.done_weight;
    let (eta_str, eta_date_str) = match (eta, today) {
        (Some(est), Some(t)) => {
            let span = eta_span(Some(est), Some(t)).unwrap_or_else(|| "unknown".to_string());
            let date = super::date_utils::date_from_unix_days(est.unix_days)
                .map(|d| d.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            (span, date)
        }
        _ => ("unknown".to_string(), "unknown".to_string()),
    };

    Ok(format!(
        "milestone: {}\n\
         ETA: {}\n\
         ETA date: {}\n\
         tasks since last milestone: {}\n\
         tasks to do: {}\n\
         tasks done: {}\n\
         tasks percentage done: {}%\n\
         weight to do: {}\n\
         weight done: {}\n\
         weight percentage done: {}%\n",
        stats.name,
        eta_str,
        eta_date_str,
        stats.total_top_level,
        tasks_todo,
        stats.done_top_level,
        stats.percentage_count(),
        super::milestone_report::format_weight(weight_todo),
        super::milestone_report::format_weight(stats.done_weight),
        stats.percentage_weight(),
    ))
}

/// Renders the "velocity: ..." / "creep: ..." text block shown for
/// `agile when --velocity`. Labels are left-padded and units/numbers are
/// aligned so the numeric value is always the last column on each line;
/// either line shows "unknown" in place of the number when its trend can't
/// be computed.
pub fn render_velocity_text(estimate: VelocityEstimate) -> String {
    format!(
        "{}\n{}\n",
        render_velocity_line("velocity:", estimate.velocity_wtpw),
        render_velocity_line("creep:", estimate.creep_wtpw),
    )
}

fn render_velocity_line(label: &str, weight_per_week: Option<f64>) -> String {
    match weight_per_week {
        Some(value) => format!("{label:<9} weight/week   {value:.2}"),
        None => format!("{label:<9} unknown"),
    }
}

/// Renders one line of the bare `agile when` report: the ETA span
/// left-padded to line up with the milestone name, matching the column
/// width used by `render_eta_text`.
fn render_when_line(name: &str, eta: Option<EtaEstimate>, today_unix_days: Option<i64>) -> String {
    let span = eta_span(eta, today_unix_days).unwrap_or_else(|| "unknown".to_string());
    format!("{span:<10}{name}\n")
}

/// Walks every task and milestone across all `.agile.md` files in backlog
/// order, invoking `on_task`/`on_milestone` as each is encountered. Both
/// callbacks receive whether the first incomplete top-level task has been
/// seen *so far* ("future" boundary state), so a milestone is "future" iff
/// its callback fires with `true` — i.e. it appears after the first
/// incomplete task in the backlog. This is the single source of truth for
/// what counts as a "future" milestone, shared by `agile when` (this
/// module) and `agile milestones` (`milestone_stats.rs`).
pub(super) fn walk_milestone_boundaries(
    root: &Path,
    mut on_task: impl FnMut(&parser::Task, bool),
    mut on_milestone: impl FnMut(&str, bool),
) {
    let mut seen_incomplete_task = false;
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
                    on_task(&task, seen_incomplete_task);
                }
                FileItem::Milestone(m) => {
                    on_milestone(&m.name, seen_incomplete_task);
                }
            }
        }
    }
}

/// Returns the names of all *future* milestones, in backlog order. See
/// [`walk_milestone_boundaries`] for what counts as "future".
fn future_milestone_names(root: &Path) -> Vec<String> {
    let mut milestones = Vec::new();
    walk_milestone_boundaries(
        root,
        |_task, _is_future| {},
        |name, is_future| {
            if is_future {
                milestones.push(name.to_string());
            }
        },
    );
    milestones
}

/// Returns the name of the `milestone_rank`-th *future* milestone (1-based).
/// See [`future_milestone_names`] for what counts as "future".
pub(super) fn milestone_name_for_rank(root: &Path, milestone_rank: usize) -> Option<String> {
    future_milestone_names(root)
        .into_iter()
        .nth(milestone_rank - 1)
}

pub(super) fn is_closed_status(status: &Status) -> bool {
    matches!(status, Status::Done | Status::Cancelled)
}

fn task_subtree_complete(task: &parser::Task) -> bool {
    is_closed_status(&task.status) && task.children.iter().all(subtask_subtree_complete)
}

fn subtask_subtree_complete(subtask: &parser::Subtask) -> bool {
    is_closed_status(&subtask.status) && subtask.children.iter().all(subtask_subtree_complete)
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
