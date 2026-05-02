//! `agile list` — list tasks or files in priority order.

use crate::cli::common::{find_task_files, parse_files, render_task};
use crate::parser::{FileItem, Status};
use std::path::{Path, PathBuf};

/// `agile list [tasks]` entry point. Prints task blocks in priority order.
///
/// `all = false` shows only top-level todo tasks; `all = true` shows every
/// task. `next`/`last` cap the output to the first/last N blocks.
pub fn run_tasks(root: &Path, next: Option<usize>, last: Option<usize>, all: bool) {
    let items = parse_files(&find_task_files(root));
    let blocks = if all {
        list_task_blocks(&items)
    } else {
        active_task_blocks(&items)
    };
    let result: String = apply_limit(blocks, next, last).into_iter().collect();
    print!("{result}");
}

/// `agile list files` entry point. Prints discovered task files.
pub fn run_files(root: &Path, next: Option<usize>, last: Option<usize>) {
    let paths = find_task_files(root);
    let limited = apply_limit(paths, next, last);
    print!("{}", format_file_list(&limited));
}

/// Returns one rendered task block per top-level [`FileItem::Task`] in `items`.
///
/// Each block contains the task's own line followed by all indented subtask lines
/// (body text is omitted). Blocks include tasks of every status: todo `[ ]`,
/// done `[x]`, and cancelled `[-]`. Milestones are skipped.
pub fn list_task_blocks(items: &[FileItem]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            FileItem::Task(task) => {
                let mut s = String::new();
                render_task(task, &mut s);
                Some(s)
            }
            FileItem::Milestone(_) => None,
        })
        .collect()
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
    items
        .iter()
        .filter_map(|item| match item {
            FileItem::Task(task) if task.status == Status::Todo => {
                let mut s = String::new();
                render_task(task, &mut s);
                Some(s)
            }
            _ => None,
        })
        .collect()
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
