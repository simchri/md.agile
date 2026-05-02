//! `agile task <action>` — task-centric subcommands.

use crate::cli::common::{find_task_files, parse_files, render_task};
use crate::parser::{FileItem, Status};
use std::path::Path;

/// `agile task next` entry point. Prints the next active task block.
pub fn run_next(root: &Path) {
    let items = parse_files(&find_task_files(root));
    print!("{}", next_task(&items));
}

/// Returns the first incomplete top-level task block from `items`.
///
/// Scans tasks in document order and returns the rendered subtree of the first
/// task whose top-level marker is todo (`[ ]`). Done and cancelled tasks are
/// skipped. Returns an empty string if every task is complete or cancelled, or
/// if there are no tasks.
pub fn next_task(items: &[FileItem]) -> String {
    for item in items {
        if let FileItem::Task(task) = item {
            if task.status == Status::Todo {
                let mut out = String::new();
                render_task(task, &mut out);
                return out;
            }
        }
    }
    String::new()
}

/// Returns the title of the first incomplete top-level task in `items`.
///
/// Same selection rule as [`next_task`] but yields just the title string. Used
/// by callers that don't need the rendered subtree (e.g. the GUI post-it).
pub fn next_task_title(items: &[FileItem]) -> Option<String> {
    items.iter().find_map(|item| match item {
        FileItem::Task(task) if task.status == Status::Todo => Some(task.title.clone()),
        _ => None,
    })
}
