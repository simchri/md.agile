use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub mod parser;

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

/// Reads and concatenates the content of all task files found under `root`.
///
/// Files are discovered by `find_task_files` (priority order) and joined with a
/// newline separator so tasks from different files remain on separate lines.
pub fn read_task_files(root: &Path) -> String {
    find_task_files(root)
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect::<Vec<_>>()
        .join("\n")
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

/// Parses `input` and returns one `String` per top-level task block.
///
/// Each block contains the task's own line followed by all indented subtask lines
/// (body text is omitted). Blocks include tasks of every status: todo `[ ]`,
/// done `[x]`, and cancelled `[-]`. Non-task content (headings, prose, milestones)
/// is ignored.
pub fn list_task_blocks(input: &str) -> Vec<String> {
    parser::parse(input)
        .into_iter()
        .filter_map(|item| match item {
            parser::FileItem::Task(task) => {
                let mut s = String::new();
                render_task(&task, &mut s);
                Some(s)
            }
            parser::FileItem::Milestone(_) => None,
        })
        .collect()
}

/// Formats all task blocks from `input` as a single string.
///
/// Convenience wrapper around `list_task_blocks` that concatenates all blocks.
/// Includes tasks of every status; use `active_task_blocks` to filter to todo only.
pub fn list_tasks(input: &str) -> String {
    list_task_blocks(input).into_iter().collect()
}

/// Returns the 1-based line number of the first incomplete top-level task in `path`.
///
/// Scans for lines starting with `- [ ]` (top-level tasks are never indented). Returns
/// `None` if the file cannot be read or contains no incomplete top-level tasks.
pub fn find_next_task_line(path: &Path) -> Option<usize> {
    let content = std::fs::read_to_string(path).ok()?;
    content.lines().enumerate().find_map(|(i, line)| {
        if line.starts_with("- [ ]") { Some(i + 1) } else { None }
    })
}

/// Returns the path of the highest-priority task file that contains at least one active task.
///
/// Files are evaluated in priority order (alphabetical by relative path). Returns `None` if no
/// file has any incomplete `[ ]` top-level tasks, or if no task files exist under `root`.
pub fn find_file_with_next_task(root: &Path) -> Option<PathBuf> {
    find_task_files(root).into_iter().find(|p| {
        std::fs::read_to_string(p)
            .map(|content| !active_task_blocks(&content).is_empty())
            .unwrap_or(false)
    })
}

/// Returns only the top-level task blocks whose top-level status is todo (`[ ]`).
///
/// Done (`[x]`) and cancelled (`[-]`) top-level tasks are excluded entirely, even
/// if they contain active subtasks. A todo parent is included with all its subtasks
/// regardless of the subtasks' individual statuses.
pub fn active_task_blocks(input: &str) -> Vec<String> {
    parser::parse(input)
        .into_iter()
        .filter_map(|item| match item {
            parser::FileItem::Task(task) if task.status == parser::Status::Todo => {
                let mut s = String::new();
                render_task(&task, &mut s);
                Some(s)
            }
            _ => None,
        })
        .collect()
}

/// Returns the first incomplete top-level task block from `input`.
///
/// Scans tasks in document order and returns the subtree of the first task whose
/// top-level marker is todo (`[ ]`). Done and cancelled tasks are skipped. Returns
/// an empty string if every task is complete or cancelled, or if there are no tasks.
pub fn next_task(input: &str) -> String {
    use parser::{FileItem, Status};
    for item in parser::parse(input) {
        if let FileItem::Task(task) = item {
            if task.status == Status::Todo {
                let mut out = String::new();
                render_task(&task, &mut out);
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
