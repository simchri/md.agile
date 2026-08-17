//! Text-only reports: the bare `agile when` list (ETA span per future
//! milestone) and `agile when --velocity`'s velocity/creep summary. Neither
//! is a chart, but both build on the same trend/ETA machinery.

use super::eta_math::{EtaEstimate, eta_for_plot};
use super::eta_text::eta_span;
use super::plot_data::build_todo_done_plot;
use super::velocity::{VelocityEstimate, require_git_repo};
use crate::cli::common::find_task_files;
use crate::parser::{self, FileItem, Status};
use std::path::Path;

/// Builds the bare `agile when` report: the ETA time span for every future
/// milestone (see [`future_milestone_names`]), one per line, in backlog
/// order — matching README.vision.md's list-mode output. A milestone whose
/// ETA can't be computed (e.g. not committed yet, or no convergent trend)
/// shows "unknown" instead of a span.
pub fn build_when_report(root: &Path) -> Result<String, String> {
    require_git_repo(root)?;
    let today = super::date_utils::today_unix_days();
    let mut out = String::new();
    for (index, name) in future_milestone_names(root).into_iter().enumerate() {
        let rank = index + 1;
        let eta = build_todo_done_plot(root, rank)
            .ok()
            .and_then(|plot| eta_for_plot(&plot, today));
        out.push_str(&render_when_line(&name, eta, today));
    }
    Ok(out)
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

/// Returns the names of all *future* milestones, in backlog order, i.e. the
/// milestones that appear after the first incomplete task in the backlog
/// (matching `agile milestones --list --next`'s semantics). Milestones that
/// only have completed tasks above them have already been reached and are
/// skipped.
fn future_milestone_names(root: &Path) -> Vec<String> {
    let mut milestones = Vec::new();
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
                }
                FileItem::Milestone(m) => {
                    if seen_incomplete_task {
                        milestones.push(m.name);
                    }
                }
            }
        }
    }
    milestones
}

/// Returns the name of the `milestone_rank`-th *future* milestone (1-based).
/// See [`future_milestone_names`] for what counts as "future".
pub(super) fn milestone_name_for_rank(root: &Path, milestone_rank: usize) -> Option<String> {
    future_milestone_names(root)
        .into_iter()
        .nth(milestone_rank - 1)
}

fn is_closed_status(status: &Status) -> bool {
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
