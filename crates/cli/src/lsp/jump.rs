/// Pure helpers for jumping to tasks in the LSP.
use crate::config::Config;
use crate::parser::{FileItem, Status, Task};
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

/// Direction to search for a task relative to a cursor position, used by
/// [`next_open_task_after`]/[`previous_open_task_before`] and their `_my`
/// counterparts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Next,
    Previous,
}

/// Find the next open task strictly after `current_line` in `current_path`
/// (preferring `current_doc_text` — the live in-editor buffer — over disk
/// content, if given), or (if none) the first open task in each subsequent
/// task file under `root`, in the same file-priority order as
/// [`find_highest_priority_open_task_in_workspace`]. Never wraps around.
/// Returns `(path, 0_based_line)`.
pub fn next_open_task_after(
    root: &Path,
    current_path: &Path,
    current_doc_text: Option<&str>,
    current_line: u32,
) -> Option<(PathBuf, u32)> {
    task_relative_to_cursor(
        root,
        current_path,
        current_doc_text,
        current_line,
        Direction::Next,
        |t| t.status == Status::Todo,
    )
}

/// Find the previous open task strictly before `current_line` in
/// `current_path` (preferring `current_doc_text` — the live in-editor
/// buffer — over disk content, if given), or (if none) the last open task in
/// each preceding task file under `root`, walking files in reverse
/// file-priority order. Never wraps around. Returns `(path, 0_based_line)`.
pub fn previous_open_task_before(
    root: &Path,
    current_path: &Path,
    current_doc_text: Option<&str>,
    current_line: u32,
) -> Option<(PathBuf, u32)> {
    task_relative_to_cursor(
        root,
        current_path,
        current_doc_text,
        current_line,
        Direction::Previous,
        |t| t.status == Status::Todo,
    )
}

/// Like [`next_open_task_after`], but additionally restricted to tasks
/// eligible for `identity` (see [`rules::is_eligible_for`]).
pub fn next_my_open_task_after(
    root: &Path,
    current_path: &Path,
    current_doc_text: Option<&str>,
    current_line: u32,
    identity: &ResolvedIdentity,
    config: &Config,
) -> Option<(PathBuf, u32)> {
    task_relative_to_cursor(
        root,
        current_path,
        current_doc_text,
        current_line,
        Direction::Next,
        |t| t.status == Status::Todo && rules::is_eligible_for(NodeRef::Task(t), identity, config),
    )
}

/// Like [`previous_open_task_before`], but additionally restricted to tasks
/// eligible for `identity` (see [`rules::is_eligible_for`]).
pub fn previous_my_open_task_before(
    root: &Path,
    current_path: &Path,
    current_doc_text: Option<&str>,
    current_line: u32,
    identity: &ResolvedIdentity,
    config: &Config,
) -> Option<(PathBuf, u32)> {
    task_relative_to_cursor(
        root,
        current_path,
        current_doc_text,
        current_line,
        Direction::Previous,
        |t| t.status == Status::Todo && rules::is_eligible_for(NodeRef::Task(t), identity, config),
    )
}

/// Shared implementation behind `next_open_task_after`/`previous_open_task_before`
/// and their `_my` counterparts: finds the closest task matching `predicate`
/// on the appropriate side of `current_line` in `current_path` — using
/// `current_doc_text` (the live in-editor buffer) instead of disk content
/// for `current_path` if given, so unsaved edits (including a
/// never-yet-saved buffer not found by `find_task_files`) are respected —
/// falling back to scanning subsequent/preceding task files (in
/// file-priority order) if none is found in the current file. Never wraps
/// around. If a match isn't found in the current file/buffer and
/// `current_path` isn't among `root`'s task files (e.g. an unsaved buffer),
/// there's no well-defined adjacent file to continue into, so this returns
/// `None` rather than guessing.
fn task_relative_to_cursor(
    root: &Path,
    current_path: &Path,
    current_doc_text: Option<&str>,
    current_line: u32,
    direction: Direction,
    predicate: impl Fn(&Task) -> bool,
) -> Option<(PathBuf, u32)> {
    let pick = |lines: Vec<u32>| match direction {
        Direction::Next => lines.into_iter().min(),
        Direction::Previous => lines.into_iter().max(),
    };

    let lines_in_current: Vec<u32> = match current_doc_text {
        Some(text) => matching_lines_in_text(text, &predicate),
        None => matching_lines_in_file(current_path, &predicate),
    }
    .into_iter()
    .filter(|&line| match direction {
        Direction::Next => line > current_line,
        Direction::Previous => line < current_line,
    })
    .collect();
    if let Some(line) = pick(lines_in_current) {
        return Some((current_path.to_path_buf(), line));
    }

    let files = crate::cli::common::find_task_files(root);
    let current_index = files.iter().position(|p| p == current_path)?;
    let other_files: Box<dyn Iterator<Item = &PathBuf>> = match direction {
        Direction::Next => Box::new(files[current_index + 1..].iter()),
        Direction::Previous => Box::new(files[..current_index].iter().rev()),
    };
    for path in other_files {
        if let Some(line) = pick(matching_lines_in_file(path, &predicate)) {
            return Some((path.clone(), line));
        }
    }
    None
}

/// Returns the 0-based line number of every task in `path` matching `predicate`.
fn matching_lines_in_file(path: &Path, predicate: &impl Fn(&Task) -> bool) -> Vec<u32> {
    crate::cli::common::parse_file(path)
        .into_iter()
        .filter_map(|item| match item {
            FileItem::Task(t) if predicate(&t) => Some((t.location.line - 1) as u32),
            _ => None,
        })
        .collect()
}

/// Returns the 0-based line number of every task in `text` matching `predicate`.
fn matching_lines_in_text(text: &str, predicate: &impl Fn(&Task) -> bool) -> Vec<u32> {
    crate::parser::parse(text, PathBuf::from("tasks.agile.md"))
        .into_iter()
        .filter_map(|item| match item {
            FileItem::Task(t) if predicate(&t) => Some((t.location.line - 1) as u32),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
#[path = "jump_tests.rs"]
mod tests;
