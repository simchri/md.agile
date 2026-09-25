/// Pure helpers for jumping to tasks in the LSP.
use crate::parser::{FileItem, Status};
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

#[cfg(test)]
#[path = "jump_tests.rs"]
mod tests;
