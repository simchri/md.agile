//! Helpers shared across CLI subcommands: file discovery, parsing, and rendering.

use crate::parser::{self, FileItem};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Finds all `*.agile.md` files anywhere under `root`, respecting `.gitignore`.
///
/// Results are sorted by their path relative to `root`. This means directory
/// components participate in the sort: `50_current/001.agile.md` outranks
/// `60_backlog/001.agile.md` even though both filenames are identical.
/// This sort order defines the global task priority across files.
pub fn find_task_files(root: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = WalkBuilder::new(root)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().map(|t| t.is_file()).unwrap_or(false)
                && e.file_name().to_string_lossy().ends_with(".agile.md")
        })
        .map(|e| e.into_path())
        .collect();

    paths.sort_by_key(|p| p.strip_prefix(root).map(|r| r.to_path_buf()).unwrap_or_else(|_| p.clone()));
    paths
}

/// Reads and parses a single `.agile.md` file.
///
/// Returns an empty vec if the file cannot be read. Every Task and Subtask in
/// the result carries the supplied `path` in its [`parser::Location`].
pub fn parse_file(path: &Path) -> Vec<FileItem> {
    match std::fs::read_to_string(path) {
        Ok(content) => parser::parse(&content, path.to_path_buf()),
        Err(_) => Vec::new(),
    }
}

/// Reads and parses every file in `paths`, concatenating the resulting items.
///
/// Each file is parsed independently so its tasks carry the file's own path in
/// their [`parser::Location`]. The order of items in the returned vec follows
/// `paths`, which the caller typically obtains from [`find_task_files`].
pub fn parse_files(paths: &[PathBuf]) -> Vec<FileItem> {
    paths.iter().flat_map(|p| parse_file(p)).collect()
}

/// Renders a top-level task and its subtree to `out` as `[<status>] <title>` lines.
///
/// The task itself is written without indentation; each successive level of
/// children is indented by two more spaces. Body text and markers are omitted —
/// only the rendered title is emitted. The output line for the task itself is
/// always terminated with a newline, so concatenating multiple rendered tasks
/// yields one task per line group.
pub fn render_task(task: &parser::Task, out: &mut String) {
    out.push_str(status_marker(&task.status));
    out.push(' ');
    out.push_str(&task.title);
    out.push('\n');
    for child in &task.children {
        render_subtask(child, 1, out);
    }
}

/// Renders a subtask and its descendants, indented by `depth * 2` spaces.
///
/// `depth` is the subtask's nesting level relative to its top-level task: the
/// immediate children of a [`parser::Task`] have depth 1, their children depth
/// 2, and so on. Used by [`render_task`] to render the recursive children.
fn render_subtask(sub: &parser::Subtask, depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
    out.push_str(status_marker(&sub.status));
    out.push(' ');
    out.push_str(&sub.title);
    out.push('\n');
    for child in &sub.children {
        render_subtask(child, depth + 1, out);
    }
}

/// Returns the textual checkbox for a [`parser::Status`]: `[ ]`, `[x]`, or `[-]`.
fn status_marker(status: &parser::Status) -> &'static str {
    match status {
        parser::Status::Todo      => "[ ]",
        parser::Status::Done      => "[x]",
        parser::Status::Cancelled => "[-]",
    }
}
