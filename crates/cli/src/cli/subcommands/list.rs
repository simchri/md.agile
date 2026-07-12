//! `agile list` — list tasks or files in priority order.

use crate::checker;
use crate::cli::common::{find_task_files, parse_files, render_task};
use crate::config::Config;
use crate::parser::{FileItem, Status};
use crate::rules::{self, NodeRef, ResolvedIdentity};
use std::path::{Path, PathBuf};

/// `agile list [tasks]` entry point. Prints task blocks in priority order.
///
/// `all = false` shows only top-level todo tasks; `all = true` shows every
/// task. `next`/`last` cap the output to the first/last N blocks. `mine`
/// further restricts the top-level tasks shown to ones that are unassigned
/// or assigned to the resolved identity (`as_user`, or the git identity if
/// `as_user` is `None`) — see [`rules::is_eligible_for`], the same
/// eligibility rule `agile task next --mine` uses.
pub fn run_tasks(
    root: &Path,
    config: &Config,
    next: Option<usize>,
    last: Option<usize>,
    all: bool,
    mine: bool,
    as_user: Option<&str>,
) {
    let identity = if mine {
        match checker::resolve_cli_identity(root, config, as_user) {
            Ok(identity) => Some(identity),
            Err(e) => {
                log::error!("{e}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let items = parse_files(&find_task_files(root));
    let blocks = task_blocks(&items, all, identity.as_ref().map(|i| (i, config)));
    let result: String = apply_limit(blocks, next, last).into_iter().collect();
    print!("{result}");
}

/// `agile list files` entry point. Prints discovered task files.
pub fn run_files(root: &Path, next: Option<usize>, last: Option<usize>) {
    let paths = find_task_files(root);
    let limited = apply_limit(paths, next, last);
    print!("{}", format_file_list(&limited));
}

/// Returns one rendered task block per top-level [`FileItem::Task`] in `items`
/// that matches the given filters.
///
/// `all = false` restricts to todo (`[ ]`) top-level tasks; `all = true`
/// includes every status. If `identity` is `Some`, tasks that aren't
/// unassigned and aren't eligible for that identity (see
/// [`rules::is_eligible_for`]) are excluded too. Milestones are always
/// skipped.
fn task_blocks(
    items: &[FileItem],
    all: bool,
    identity: Option<(&ResolvedIdentity, &Config)>,
) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            FileItem::Task(task) => {
                if !all && task.status != Status::Todo {
                    return None;
                }
                if let Some((identity, config)) = identity {
                    if !rules::is_eligible_for(NodeRef::Task(task), identity, config) {
                        return None;
                    }
                }
                let mut s = String::new();
                render_task(task, &mut s);
                Some(s)
            }
            FileItem::Milestone(_) => None,
        })
        .collect()
}

/// Returns one rendered task block per top-level [`FileItem::Task`] in `items`.
///
/// Each block contains the task's own line followed by all indented subtask lines
/// (body text is omitted). Blocks include tasks of every status: todo `[ ]`,
/// done `[x]`, and cancelled `[-]`. Milestones are skipped.
pub fn list_task_blocks(items: &[FileItem]) -> Vec<String> {
    task_blocks(items, true, None)
}

/// Concatenates all task blocks from `items` into a single string.
///
/// Convenience wrapper around [`list_task_blocks`]. Includes tasks of every
/// status; use [`active_task_blocks`] to filter to todo only.
pub fn list_tasks(items: &[FileItem]) -> String {
    list_task_blocks(items).into_iter().collect()
}

/// Returns only the top-level task blocks whose top-level status is todo (`[ ]`).
///
/// Done (`[x]`) and cancelled (`[-]`) top-level tasks are excluded entirely, even
/// if they contain active subtasks. A todo parent is included with all its subtasks
/// regardless of the subtasks' individual statuses.
pub fn active_task_blocks(items: &[FileItem]) -> Vec<String> {
    task_blocks(items, false, None)
}

/// Formats a list of task file paths into a display string.
///
/// Each line is `<filename>  <full-path>`, terminated with a newline.
/// Files are shown in the order provided; sorting is the caller's responsibility.
pub fn format_file_list(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            format!("{name}  {}\n", p.display())
        })
        .collect()
}

/// Caps `items` to the first `next` or last `last` entries (if either is set).
///
/// `next` takes precedence if both are provided. With neither set, returns
/// `items` unchanged.
pub fn apply_limit<T>(items: Vec<T>, next: Option<usize>, last: Option<usize>) -> Vec<T> {
    match (next, last) {
        (Some(n), _) => items.into_iter().take(n).collect(),
        (_, Some(n)) => {
            let skip = items.len().saturating_sub(n);
            items.into_iter().skip(skip).collect()
        }
        (None, None) => items,
    }
}
