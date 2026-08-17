//! Velocity/creep estimation and status-transition tracking: turning a
//! milestone's todo/done plot into a velocity estimate, and matching task
//! nodes between two parses of the backlog (old vs. new) to detect status
//! transitions (e.g. todo -> done).

use super::plot_data::build_todo_done_plot;
use super::trend::{DAYS_PER_WEEK, compute_milestone_trends};
use crate::git;
use crate::parser::{self, FileItem, Status};
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
struct FlatNode {
    key: TransitionKey,
    status: Status,
    depth: usize,
    indent: usize,
    title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransitionKey {
    pub path: Vec<String>,
    pub occurrence: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusTransition {
    pub key: TransitionKey,
    pub old_status: Option<Status>,
    pub new_status: Status,
    pub depth: usize,
    pub indent: usize,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FallbackSignature {
    depth: usize,
    title: String,
    parent_title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityEstimate {
    /// Slope of the whole-project done-weight trend line, in weight/week:
    /// how fast completed weight is accumulating.
    pub velocity_wtpw: Option<f64>,
    /// Slope of the whole-project total-weight trend line, in weight/week:
    /// how fast the backlog itself is growing (new/expanded work).
    pub creep_wtpw: Option<f64>,
}

/// Estimates current velocity and creep (see [`VelocityEstimate`]), scoped
/// to the next milestone (rank 1) — exactly the same trend lines
/// [`build_todo_done_plot`]/`agile when --plot` would fit and show for that
/// milestone.
///
/// Returns an error when `root` isn't a git repository, or when there's no
/// future milestone to scope to (see [`build_todo_done_plot`]).
pub fn estimate_velocity(root: &Path) -> Result<VelocityEstimate, String> {
    estimate_velocity_with_window(root, 1, None)
}

/// Like [`estimate_velocity`], but scoped to a caller-provided milestone
/// rank (see [`build_todo_done_plot`]) and, optionally, further restricted
/// to a caller-provided trailing window (in days) — matching the
/// `--next`/`--last` flags on `agile when --velocity`. `window_days: None`
/// applies no day-based restriction, using the milestone's whole history.
///
/// Velocity and creep are the slopes of the exact same done-weight/
/// total-weight trend lines `agile when --plot` fits and draws for the same
/// milestone (see [`compute_milestone_trends`]) — both expressed in weight/week.
/// Either metric independently resolves to `None` when its trend line can't
/// be computed (fewer than two distinct points in the window).
pub fn estimate_velocity_with_window(
    root: &Path,
    milestone_rank: usize,
    window_days: Option<u32>,
) -> Result<VelocityEstimate, String> {
    require_git_repo(root)?;
    if window_days == Some(0) {
        return Ok(VelocityEstimate {
            velocity_wtpw: None,
            creep_wtpw: None,
        });
    }

    let mut plot = build_todo_done_plot(root, milestone_rank)?;

    let today = super::date_utils::today_unix_days();
    if let Some(cutoff) = window_days.and_then(|days| today.map(|t| t - i64::from(days))) {
        plot.points
            .retain(|p| super::date_utils::unix_days_from_date(p.date) >= cutoff);
    }

    let trends = compute_milestone_trends(&plot);

    Ok(VelocityEstimate {
        velocity_wtpw: trends.done_trend.map(|t| t.slope_wtpd * DAYS_PER_WEEK),
        creep_wtpw: trends.total_trend.map(|t| t.slope_wtpd * DAYS_PER_WEEK),
    })
}

/// Ensures `root` is a git repository, returning a consistent error message
/// (shared by every `agile when` entry point that needs commit history).
pub(super) fn require_git_repo(root: &Path) -> Result<(), String> {
    if !git::is_git_repo(root) {
        return Err("`agile when` requires a git repository".to_string());
    }
    Ok(())
}

// Currently unused now that velocity estimation is disabled (history cache removal),
// but retained for reuse once velocity is rebuilt on top of the lifecycle cache.
#[allow(dead_code)]
pub(crate) fn completion_weight_delta(
    old_items: &[FileItem],
    new_items: &[FileItem],
) -> (f64, usize) {
    let transitions = status_transitions(old_items, new_items);
    let mut completed_weight = 0.0f64;
    let mut completion_events = 0usize;
    for t in transitions {
        if t.old_status == Some(Status::Todo) && t.new_status == Status::Done {
            completion_events += 1;
            completed_weight += weight_for_depth(t.depth);
        }
    }
    (completed_weight, completion_events)
}

/// Returns path+occurrence-matched status transitions from `old_items` to
/// `new_items`.
pub fn status_transitions(old_items: &[FileItem], new_items: &[FileItem]) -> Vec<StatusTransition> {
    let old_nodes = flatten_nodes(old_items);
    let new_nodes = flatten_nodes(new_items);
    let old_by_key: HashMap<TransitionKey, FlatNode> =
        old_nodes.into_iter().map(|n| (n.key.clone(), n)).collect();

    let mut matched_old = HashSet::new();
    let mut transitions = Vec::with_capacity(new_nodes.len());
    let mut unmatched_new = Vec::new();
    for new in new_nodes {
        let old_status = old_by_key.get(&new.key).map(|old| {
            matched_old.insert(new.key.clone());
            old.status.clone()
        });
        if old_status.is_none() {
            unmatched_new.push(new.clone());
        }
        transitions.push(StatusTransition {
            key: new.key,
            old_status,
            new_status: new.status,
            depth: new.depth,
            indent: new.indent,
            title: new.title,
        });
    }

    // Fallback matcher: when strict path+occurrence fails (e.g. ancestor title
    // churn), match uniquely by local structural signature.
    let mut old_unmatched_by_sig: HashMap<FallbackSignature, Vec<TransitionKey>> = HashMap::new();
    for (key, old) in &old_by_key {
        if matched_old.contains(key) {
            continue;
        }
        old_unmatched_by_sig
            .entry(fallback_signature(old))
            .or_default()
            .push(key.clone());
    }

    let mut consumed_old_fallback = HashSet::new();
    for t in &mut transitions {
        if t.old_status.is_some() {
            continue;
        }
        let sig = FallbackSignature {
            depth: t.depth,
            title: t.title.clone(),
            parent_title: parent_title_from_path(&t.key.path),
        };
        let Some(candidates) = old_unmatched_by_sig.get(&sig) else {
            continue;
        };
        // Conservative: only use fallback when there is one unambiguous old node.
        let available: Vec<&TransitionKey> = candidates
            .iter()
            .filter(|k| !consumed_old_fallback.contains(*k))
            .collect();
        if available.len() != 1 {
            continue;
        }
        let key = available[0];
        let Some(old) = old_by_key.get(key) else {
            continue;
        };
        consumed_old_fallback.insert(key.clone());
        t.old_status = Some(old.status.clone());
    }

    transitions
}

fn flatten_nodes(items: &[FileItem]) -> Vec<FlatNode> {
    let mut raw = Vec::new();
    for item in items {
        let FileItem::Task(task) = item else {
            continue;
        };
        let path = vec![task.title.clone()];
        raw.push((
            path.clone(),
            task.status.clone(),
            1usize,
            task.indent,
            task.title.clone(),
        ));
        flatten_subtasks(&mut raw, &path, &task.children, 2);
    }

    let mut occurrence_index: HashMap<Vec<String>, usize> = HashMap::new();
    raw.into_iter()
        .map(|(path, status, depth, indent, title)| {
            let occurrence = occurrence_index.entry(path.clone()).or_insert(0);
            let key = TransitionKey {
                path: path.clone(),
                occurrence: *occurrence,
            };
            *occurrence += 1;
            FlatNode {
                key,
                status,
                depth,
                indent,
                title,
            }
        })
        .collect()
}

fn flatten_subtasks(
    out: &mut Vec<(Vec<String>, Status, usize, usize, String)>,
    parent_path: &[String],
    children: &[parser::Subtask],
    depth: usize,
) {
    for child in children {
        let mut path = parent_path.to_vec();
        path.push(child.title.clone());
        out.push((
            path.clone(),
            child.status.clone(),
            depth,
            child.indent,
            child.title.clone(),
        ));
        flatten_subtasks(out, &path, &child.children, depth + 1);
    }
}

pub(crate) fn weight_for_depth(depth: usize) -> f64 {
    1.0 / (depth as f64)
}

fn fallback_signature(node: &FlatNode) -> FallbackSignature {
    FallbackSignature {
        depth: node.depth,
        title: node.title.clone(),
        parent_title: parent_title_from_path(&node.key.path),
    }
}

fn parent_title_from_path(path: &[String]) -> Option<String> {
    path.len().checked_sub(2).map(|idx| path[idx].clone())
}

#[cfg(test)]
#[path = "velocity_tests.rs"]
mod tests;
