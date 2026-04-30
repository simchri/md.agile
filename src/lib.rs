use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub mod checker;
pub mod formatter;
pub mod lsp;
pub mod parser;
pub mod rules;

use parser::{FileItem, Status};
use rules::Issue;

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

/// Returns one rendered task block per top-level [`parser::FileItem::Task`] in `items`.
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

/// Returns the 1-based line number of the first incomplete top-level task in `path`.
///
/// Reads and parses `path`, then returns the [`parser::Location::line`] of the
/// first top-level Task whose status is [`Status::Todo`]. Returns `None` if the
/// file cannot be read or contains no incomplete top-level tasks.
pub fn find_next_task_line(path: &Path) -> Option<usize> {
    parse_file(path).into_iter().find_map(|item| match item {
        FileItem::Task(t) if t.status == Status::Todo => Some(t.location.line),
        _ => None,
    })
}

/// Returns the path of the highest-priority task file that contains at least one active task.
///
/// Files are evaluated in priority order (alphabetical by relative path). Returns `None` if no
/// file has any incomplete `[ ]` top-level tasks, or if no task files exist under `root`.
pub fn find_file_with_next_task(root: &Path) -> Option<PathBuf> {
    find_task_files(root).into_iter().find(|p| file_has_active_task(p))
}

fn file_has_active_task(path: &Path) -> bool {
    parse_file(path).iter().any(|item| {
        matches!(item, FileItem::Task(t) if t.status == Status::Todo)
    })
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

/// Renders a top-level task and its subtree to `out` as `[<status>] <title>` lines.
///
/// The task itself is written without indentation; each successive level of
/// children is indented by two more spaces. Body text and markers are omitted —
/// only the rendered title is emitted. The output line for the task itself is
/// always terminated with a newline, so concatenating multiple rendered tasks
/// yields one task per line group.
fn render_task(task: &parser::Task, out: &mut String) {
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

/// Formats an Issue into ESLint-style output with source context.
///
/// Reads the source file, extracts the relevant lines, and formats the output
/// with color codes (for terminals that support them) and helpful context.
pub fn format_issue(issue: &Issue) -> String {
    formatter::format_issue(issue)
}
