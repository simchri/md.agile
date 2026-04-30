use ignore::WalkBuilder;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
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

#[derive(Clone, Copy, PartialEq)]
enum ItemKind {
    Todo,
    Done,
    MaybeCancel,
}

struct ItemState {
    kind: ItemKind,
    title_written: bool,
    buf: String,
}

impl ItemState {
    fn new() -> Self {
        Self { kind: ItemKind::MaybeCancel, title_written: false, buf: String::new() }
    }
}

// Returns true if `s` could still be the start of "[-] "
fn is_cancel_prefix(s: &str) -> bool {
    "[-] ".starts_with(s)
}

fn write_task_text(out: &mut String, item: &mut ItemState, text: &str, list_depth: usize) {
    if item.title_written {
        return;
    }
    let indent = "  ".repeat(list_depth - 1);
    match item.kind {
        ItemKind::Todo => {
            out.push_str(&format!("{}[ ] {}\n", indent, text));
            item.title_written = true;
        }
        ItemKind::Done => {
            out.push_str(&format!("{}[x] {}\n", indent, text));
            item.title_written = true;
        }
        ItemKind::MaybeCancel => {
            item.buf.push_str(text);
            if let Some(rest) = item.buf.strip_prefix("[-] ") {
                out.push_str(&format!("{}[-] {}\n", indent, rest));
                item.title_written = true;
            } else if !is_cancel_prefix(&item.buf) {
                item.title_written = true;
            }
        }
    }
}

fn make_parser(input: &str) -> Parser<'_> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TASKLISTS);
    Parser::new_ext(input, opts)
}

/// Parses `input` and returns one `String` per top-level task block.
///
/// Each block contains the task's own line followed by all indented subtask lines
/// (body text is omitted). Blocks include tasks of every status: todo `[ ]`,
/// done `[x]`, and cancelled `[-]`. Non-task content (headings, prose) is ignored.
pub fn list_task_blocks(input: &str) -> Vec<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut list_depth: usize = 0;
    let mut stack: Vec<ItemState> = Vec::new();

    for event in make_parser(input) {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth -= 1,
            Event::Start(Tag::Item) => stack.push(ItemState::new()),
            Event::End(TagEnd::Item) => {
                let at_top = list_depth == 1 && stack.len() == 1;
                stack.pop();
                if at_top && !current.is_empty() {
                    blocks.push(std::mem::take(&mut current));
                }
            }
            Event::TaskListMarker(checked) => {
                if let Some(item) = stack.last_mut() {
                    item.kind = if checked { ItemKind::Done } else { ItemKind::Todo };
                }
            }
            Event::Text(text) => {
                if let Some(item) = stack.last_mut() {
                    write_task_text(&mut current, item, &text, list_depth);
                }
            }
            _ => {}
        }
    }

    blocks
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
    list_task_blocks(input)
        .into_iter()
        .filter(|b| b.starts_with("[ ]"))
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

fn render_task(task: &parser::Task, out: &mut String) {
    out.push_str(status_marker(&task.status));
    out.push(' ');
    out.push_str(&task.title);
    out.push('\n');
    for child in &task.children {
        render_subtask(child, 1, out);
    }
}

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

fn status_marker(status: &parser::Status) -> &'static str {
    match status {
        parser::Status::Todo      => "[ ]",
        parser::Status::Done      => "[x]",
        parser::Status::Cancelled => "[-]",
    }
}
