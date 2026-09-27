/// Pure helpers for jumping to tasks in the LSP.
use crate::config::Config;
use crate::parser::{FileItem, Subtask, Task};
use crate::rules::{self, NodeRef, ResolvedIdentity};
use std::path::{Path, PathBuf};

/// Returns the 0-based line of the actionable (sub)task within `task`'s own
/// subtree — the same concrete unit [`rules::find_next_actionable`] would
/// find for `agile task next`/`done` — or `None` if there isn't one.
///
/// When `identity` is `Some`, `task`'s whole subtree is first gated by
/// [`rules::is_eligible_for`] (which correctly handles an assignment on an
/// ancestor claiming its entire subtree, recursing arbitrarily deep) before
/// descending: [`rules::find_next_actionable`]'s own per-node identity
/// check only looks at each node's *own* markers as it descends, so without
/// this outer gate it could incorrectly land inside a subtree an ancestor's
/// assignment had already excluded `identity` from entirely.
fn actionable_line(task: &Task, identity: Option<(&ResolvedIdentity, &Config)>) -> Option<u32> {
    if let Some((identity, config)) = identity {
        if !rules::is_eligible_for(NodeRef::Task(task), identity, config) {
            return None;
        }
    }
    let no_siblings: &[Subtask] = &[];
    rules::find_next_actionable(NodeRef::Task(task), no_siblings, identity)
        .map(|node| (node.location().line - 1) as u32)
}

/// Search files in workspace priority order, substituting the current editor
/// buffer for its on-disk counterpart. A buffer not found by file discovery
/// is searched after discovered files, as it has no workspace priority rank.
pub fn highest_priority_open_task(
    root: Option<&Path>,
    current_path: &Path,
    current_doc_text: Option<&str>,
    identity: Option<(&ResolvedIdentity, &Config)>,
) -> Option<(PathBuf, u32)> {
    search_task_files(
        root,
        current_path,
        current_doc_text,
        Search::Highest,
        identity,
    )
}

/// Direction to search for a task relative to a cursor position, used by
/// [`next_open_task_after`]/[`previous_open_task_before`] and their `_my`
/// counterparts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Next,
    Previous,
}

#[derive(Clone, Copy)]
enum Search {
    Highest,
    Relative {
        current_line: u32,
        direction: Direction,
    },
}

/// Find the next open (sub)task strictly after `current_line` in
/// `current_path` (preferring `current_doc_text` — the live in-editor
/// buffer — over disk content, if given), or (if none) the first one in
/// each subsequent task file under `root`, in the same file-priority order
/// as [`highest_priority_open_task`]. Never wraps around.
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
        None,
    )
}

/// Find the previous open (sub)task strictly before `current_line` in
/// `current_path` (preferring `current_doc_text` — the live in-editor
/// buffer — over disk content, if given), or (if none) the last one in each
/// preceding task file under `root`, walking files in reverse file-priority
/// order. Never wraps around. Returns `(path, 0_based_line)`.
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
        None,
    )
}

/// Like [`next_open_task_after`], but additionally restricted to (sub)tasks
/// eligible for `identity` (see [`actionable_line`]).
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
        Some((identity, config)),
    )
}

/// Like [`previous_open_task_before`], but additionally restricted to
/// (sub)tasks eligible for `identity` (see [`actionable_line`]).
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
        Some((identity, config)),
    )
}

/// Shared implementation behind `next_open_task_after`/`previous_open_task_before`
/// and their `_my` counterparts: finds the closest actionable (sub)task
/// (see [`actionable_line`]) on the appropriate side of `current_line` in
/// `current_path` — using `current_doc_text` (the live in-editor buffer)
/// instead of disk content for `current_path` if given, so unsaved edits
/// (including a never-yet-saved buffer not found by `find_task_files`) are
/// respected — falling back to scanning subsequent/preceding task files (in
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
    identity: Option<(&ResolvedIdentity, &Config)>,
) -> Option<(PathBuf, u32)> {
    search_task_files(
        Some(root),
        current_path,
        current_doc_text,
        Search::Relative {
            current_line,
            direction,
        },
        identity,
    )
}

/// Supply paths in search order; the current path is substituted from the live
/// buffer by `matching_lines`, regardless of its position in this sequence.
fn ordered_task_paths(
    root: Option<&Path>,
    current_path: &Path,
    current_doc_text: Option<&str>,
    search: Search,
) -> Vec<PathBuf> {
    let Some(root) = root else {
        return if matches!(search, Search::Highest) && current_doc_text.is_some() {
            vec![current_path.to_path_buf()]
        } else {
            vec![]
        };
    };
    let mut files = crate::cli::common::find_task_files(root);
    let current_index = files.iter().position(|path| path == current_path);
    match search {
        Search::Highest => {
            if current_index.is_none() && current_doc_text.is_some() {
                files.push(current_path.to_path_buf());
            }
            files
        }
        Search::Relative { direction, .. } => {
            let mut ordered = vec![current_path.to_path_buf()];
            if let Some(index) = current_index {
                match direction {
                    Direction::Next => ordered.extend(files.drain(index + 1..)),
                    Direction::Previous => ordered.extend(files.drain(..index).rev()),
                }
            }
            ordered
        }
    }
}

fn search_task_files(
    root: Option<&Path>,
    current_path: &Path,
    current_doc_text: Option<&str>,
    search: Search,
    identity: Option<(&ResolvedIdentity, &Config)>,
) -> Option<(PathBuf, u32)> {
    ordered_task_paths(root, current_path, current_doc_text, search)
        .into_iter()
        .find_map(|path| {
            let lines = matching_lines(&path, current_path, current_doc_text, identity);
            let line = match search {
                Search::Highest => lines.into_iter().next(),
                Search::Relative {
                    current_line,
                    direction,
                } => {
                    let candidates = lines.into_iter().filter(|line| {
                        path != current_path
                            || match direction {
                                Direction::Next => *line > current_line,
                                Direction::Previous => *line < current_line,
                            }
                    });
                    match direction {
                        Direction::Next => candidates.min(),
                        Direction::Previous => candidates.max(),
                    }
                }
            }?;
            Some((path, line))
        })
}

/// Resolve each workspace path from its live buffer if it is the current
/// document; otherwise read from disk.
fn matching_lines(
    path: &Path,
    current_path: &Path,
    current_doc_text: Option<&str>,
    identity: Option<(&ResolvedIdentity, &Config)>,
) -> Vec<u32> {
    if path == current_path {
        if let Some(text) = current_doc_text {
            return matching_lines_in_text(text, path, identity);
        }
    }
    actionable_lines(crate::cli::common::parse_file(path), identity)
}

/// Returns the actionable line (see [`actionable_line`]) of every top-level
/// task in `text`, one entry per top-level task that has one.
fn matching_lines_in_text(
    text: &str,
    path: &Path,
    identity: Option<(&ResolvedIdentity, &Config)>,
) -> Vec<u32> {
    actionable_lines(crate::parser::parse(text, path.to_path_buf()), identity)
}

fn actionable_lines(
    items: Vec<FileItem>,
    identity: Option<(&ResolvedIdentity, &Config)>,
) -> Vec<u32> {
    items
        .into_iter()
        .filter_map(|item| match item {
            FileItem::Task(t) => actionable_line(&t, identity),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
#[path = "jump_tests.rs"]
mod tests;
