/// Pure helpers for jumping to tasks in the LSP.
use crate::config::Config;
use crate::parser::{FileItem, Status};
use crate::rules::{self, NodeRef, ResolvedIdentity};
use std::path::{Path, PathBuf};

/// Find the 0-based line number of the highest-priority open task in the document text.
///
/// Location lines in `FileItem::Task` are 1-based, so this returns `line - 1`.
pub fn highest_priority_open_task_line(doc_text: &str) -> Option<u32> {
    let items = crate::parser::parse(doc_text, PathBuf::from("tasks.agile.md"));
    items.into_iter().find_map(|item| match item {
        FileItem::Task(t) if t.status == Status::Todo => Some((t.location.line - 1) as u32),
        _ => None,
    })
}

/// Find the highest-priority open task across all task files under `root`,
/// returning `(path, 0_based_line)`.
pub fn find_highest_priority_open_task_in_workspace(root: &Path) -> Option<(PathBuf, u32)> {
    crate::cli::common::find_task_files(root)
        .into_iter()
        .find_map(|path| {
            let items = crate::cli::common::parse_file(&path);
            items.into_iter().find_map(|item| match item {
                FileItem::Task(t) if t.status == Status::Todo => {
                    Some((path.clone(), (t.location.line - 1) as u32))
                }
                _ => None,
            })
        })
}

/// Find the 0-based line number of the highest-priority open task in the
/// document text that's eligible for `identity` (see [`rules::is_eligible_for`]).
pub fn my_highest_priority_open_task_line(
    doc_text: &str,
    identity: &ResolvedIdentity,
    config: &Config,
) -> Option<u32> {
    let items = crate::parser::parse(doc_text, PathBuf::from("tasks.agile.md"));
    items.into_iter().find_map(|item| match item {
        FileItem::Task(t)
            if t.status == Status::Todo
                && rules::is_eligible_for(NodeRef::Task(&t), identity, config) =>
        {
            Some((t.location.line - 1) as u32)
        }
        _ => None,
    })
}

/// Find the highest-priority open task, eligible for `identity`, across all
/// task files under `root`, returning `(path, 0_based_line)`.
pub fn find_my_highest_priority_open_task_in_workspace(
    root: &Path,
    identity: &ResolvedIdentity,
    config: &Config,
) -> Option<(PathBuf, u32)> {
    crate::cli::common::find_task_files(root)
        .into_iter()
        .find_map(|path| {
            let items = crate::cli::common::parse_file(&path);
            items.into_iter().find_map(|item| match item {
                FileItem::Task(t)
                    if t.status == Status::Todo
                        && rules::is_eligible_for(NodeRef::Task(&t), identity, config) =>
                {
                    Some((path.clone(), (t.location.line - 1) as u32))
                }
                _ => None,
            })
        })
}

#[cfg(test)]
#[path = "jump_tests.rs"]
mod tests;
