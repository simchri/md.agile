//! Helpers shared across CLI subcommands: file discovery, parsing, and rendering.

use crate::config::Config;
use crate::parser::{self, FileItem};
use crate::rules::{self, NodeRef, ResolvedIdentity};
use ignore::WalkBuilder;
use log::{debug, warn};
use std::path::{Path, PathBuf};

pub trait IsFileOrSymlink {
    /// Returns `true` if the path points to a file or a symbolic link.
    ///
    /// # Examples
    ///
    /// ```
    /// use mdagile::cli::common::IsFileOrSymlink;
    /// use std::path::PathBuf;
    /// let path = PathBuf::from("some_file.txt");
    /// let result = path.is_file_or_symlink();
    /// ```
    fn is_file_or_symlink(&self) -> bool;
}

/// Implementation of `IsFileOrSymlink` for `PathBuf`.
impl IsFileOrSymlink for std::path::PathBuf {
    /// Checks if the `PathBuf` is a file or a symbolic link.
    fn is_file_or_symlink(&self) -> bool {
        use std::fs;
        match fs::symlink_metadata(self) {
            Ok(metadata) => {
                if metadata.is_file() {
                    true
                } else if metadata.file_type().is_symlink() {
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }
}

/// Finds all `*.agile.md` files anywhere under `root`, respecting `.gitignore`.
///
/// Results are sorted by their path relative to `root`. This means directory
/// components participate in the sort: `50_current/001.agile.md` outranks
/// `60_backlog/001.agile.md` even though both filenames are identical.
/// This sort order defines the global task priority across files.
pub fn find_task_files(root: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = WalkBuilder::new(root)
        .follow_links(true)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path().to_path_buf();
            path.is_file_or_symlink() && e.file_name().to_string_lossy().ends_with(".agile.md")
        })
        .map(|e| e.into_path())
        .collect();

    paths.sort_by_key(|p| {
        p.strip_prefix(root)
            .map(|r| r.to_path_buf())
            .unwrap_or_else(|_| p.clone())
    });

    for p in &paths {
        debug!("found task file: {}", p.display());
    }
    debug!("total task files: {}", paths.len());

    paths
}

/// Reads and parses a single `.agile.md` file.
///
/// Returns an empty vec if the file cannot be read. Every Task and Subtask in
/// the result carries the supplied `path` in its [`parser::Location`].
pub fn parse_file(path: &Path) -> Vec<FileItem> {
    match std::fs::read_to_string(path) {
        Ok(content) => parser::parse(&content, path.to_path_buf()),
        Err(e) => {
            warn!("could not read {}: {e}", path.display());
            Vec::new()
        }
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
    render_node_as_root(&task.status, &task.title, &task.children, out);
}

/// Same as [`render_task`], but bolds the line of the first `Todo` *leaf*
/// (a node with no children of its own) encountered in document order — the
/// concrete next actionable task within the printed subtree — and, if
/// `include_body` is true, also prints each node's body lines indented one
/// level deeper than the node itself. If `identity` is `Some`, only a leaf
/// [`rules::is_eligible_for`] that identity is considered a bolding
/// candidate — leaves assigned to someone else are skipped over (but still
/// printed, unbolded) so the highlighted line is one `identity` can
/// actually act on. Used exclusively by `agile task next` to highlight
/// which line is actually "next" (and, with `--full`, show task bodies)
/// when a whole task tree (with already-done siblings/subtasks) is shown.
///
/// Every task/subtask line is additionally prefixed with its dotted task
/// address (see [`crate::cli::subcommands::task::parse_address`]), rooted at
/// `root_number` — e.g. a child of a root numbered `"1"` is numbered `"1.1"`,
/// its own children `"1.1.1"`, `"1.1.2"`, and so on — so the printed number
/// can be fed straight back into `agile task done`/`agile task next`.
/// Number columns are right-padded with spaces so every `[<status>]` marker
/// in the block still lines up in one column regardless of each number's
/// width; body lines (when printed) are padded the same way so they stay
/// aligned under that column too.
pub(crate) fn render_task_highlighting_next_leaf(
    task: &parser::Task,
    include_body: bool,
    identity: Option<(&ResolvedIdentity, &Config)>,
    no_markup: bool,
    root_number: &str,
    out: &mut String,
) {
    let mut found = false;
    let mut lines = Vec::new();
    collect_node_lines_highlighting_next_leaf(
        NodeRef::Task(task),
        &task.body,
        &task.children,
        include_body,
        identity,
        no_markup,
        root_number,
        &mut lines,
        &mut found,
    );
    write_aligned_lines(&lines, out);
}

/// Renders a subtask as if it were the root of its own tree (no leading
/// indentation for `sub` itself, children indented by two spaces per level),
/// bolding the first `Todo` leaf like [`render_task_highlighting_next_leaf`]
/// does (subject to the same `identity` eligibility restriction), and
/// likewise printing body lines when `include_body` is true. Used when a
/// dotted task address (e.g. `agile task next 1.2`) points at a specific
/// subtask rather than a whole top-level task — the addressed subtask is
/// displayed as its own root instead of nested under its ancestors.
///
/// Numbering works exactly like [`render_task_highlighting_next_leaf`]:
/// `root_number` is the addressed subtask's own dotted address (e.g. `"1.2"`
/// for `agile task next 1.2`), and its children continue from there
/// (`"1.2.1"`, `"1.2.2"`, ...).
pub(crate) fn render_subtask_as_root_highlighting_next_leaf(
    sub: &parser::Subtask,
    include_body: bool,
    identity: Option<(&ResolvedIdentity, &Config)>,
    no_markup: bool,
    root_number: &str,
    out: &mut String,
) {
    let mut found = false;
    let mut lines = Vec::new();
    collect_node_lines_highlighting_next_leaf(
        NodeRef::Subtask(sub),
        &sub.body,
        &sub.children,
        include_body,
        identity,
        no_markup,
        root_number,
        &mut lines,
        &mut found,
    );
    write_aligned_lines(&lines, out);
}

fn render_node_as_root(
    status: &parser::Status,
    title: &str,
    children: &[parser::Subtask],
    out: &mut String,
) {
    out.push_str(status_marker(status));
    out.push(' ');
    out.push_str(title);
    out.push('\n');
    for child in children {
        render_subtask(child, 1, out);
    }
}

/// One line of a rendered task/subtree, prior to number-column alignment
/// (see [`write_aligned_lines`]).
enum TreeLine {
    /// A numbered task/subtask line: its dotted address, and the already
    /// fully-formatted (indented, markup-wrapped) rest of the line.
    Numbered(String, String),
    /// A body line, printed as-is once padded to align under the number
    /// column.
    Body(String),
}

/// Writes `lines` to `out`, right-padding every [`TreeLine::Numbered`]'s
/// address with spaces to the width of the longest address in `lines`, so
/// every `[<status>]` marker ends up in the same column regardless of how
/// long its own number is. [`TreeLine::Body`] lines carry no number of their
/// own but are padded by the same width so they still line up under that
/// column.
fn write_aligned_lines(lines: &[TreeLine], out: &mut String) {
    let width = lines
        .iter()
        .filter_map(|line| match line {
            TreeLine::Numbered(number, _) => Some(number.chars().count()),
            TreeLine::Body(_) => None,
        })
        .max()
        .unwrap_or(0);
    for line in lines {
        match line {
            TreeLine::Numbered(number, content) => {
                out.push_str(number);
                for _ in 0..(width - number.chars().count() + 1) {
                    out.push(' ');
                }
                out.push_str(content);
            }
            TreeLine::Body(text) => {
                for _ in 0..(width + 1) {
                    out.push(' ');
                }
                out.push_str(text);
            }
        }
        out.push('\n');
    }
}

fn collect_node_lines_highlighting_next_leaf(
    node: NodeRef,
    body: &[String],
    children: &[parser::Subtask],
    include_body: bool,
    identity: Option<(&ResolvedIdentity, &Config)>,
    no_markup: bool,
    number: &str,
    lines: &mut Vec<TreeLine>,
    found: &mut bool,
) {
    lines.push(TreeLine::Numbered(
        number.to_string(),
        node_line_content(node, &[], identity, no_markup, 0, found),
    ));
    if include_body {
        collect_body_lines(body, lines);
    }
    for (i, child) in children.iter().enumerate() {
        let child_number = format!("{number}.{}", i + 1);
        collect_subtask_lines_highlighting_next_leaf(
            child,
            1,
            children,
            include_body,
            identity,
            no_markup,
            &child_number,
            lines,
            found,
        );
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

/// Same as [`render_subtask`], but bolds the first `Todo` leaf encountered
/// in document order (tracked via `found`), like
/// [`render_task_highlighting_next_leaf`] does for the root, subject to the
/// same `identity` eligibility restriction, and prints body lines when
/// `include_body` is true. `siblings` is the slice `sub` was found in
/// (its parent's children), needed to check whether `sub` is order-blocked
/// — see [`rules::is_next_eligible_leaf`]. `number` is `sub`'s own dotted
/// task address; its children continue from there (see
/// [`collect_node_lines_highlighting_next_leaf`]).
fn collect_subtask_lines_highlighting_next_leaf(
    sub: &parser::Subtask,
    depth: usize,
    siblings: &[parser::Subtask],
    include_body: bool,
    identity: Option<(&ResolvedIdentity, &Config)>,
    no_markup: bool,
    number: &str,
    lines: &mut Vec<TreeLine>,
    found: &mut bool,
) {
    lines.push(TreeLine::Numbered(
        number.to_string(),
        node_line_content(
            NodeRef::Subtask(sub),
            siblings,
            identity,
            no_markup,
            depth,
            found,
        ),
    ));
    if include_body {
        collect_body_lines(&sub.body, lines);
    }
    for (i, child) in sub.children.iter().enumerate() {
        let child_number = format!("{number}.{}", i + 1);
        collect_subtask_lines_highlighting_next_leaf(
            child,
            depth + 1,
            &sub.children,
            include_body,
            identity,
            no_markup,
            &child_number,
            lines,
            found,
        );
    }
}

/// Turns each line of `body` into a [`TreeLine::Body`]. Body lines are
/// stored with their original source indentation intact (see
/// [`parser::Task::body`]), so no extra indentation is added here.
fn collect_body_lines(body: &[String], lines: &mut Vec<TreeLine>) {
    for line in body {
        lines.push(TreeLine::Body(line.clone()));
    }
}

/// Builds one `[<status>] <title>` line (indented by `depth * 2` spaces),
/// marking it as the concrete "next" actionable line — the first one seen
/// so far (`*found` not yet set) for which [`rules::is_next_eligible_leaf`]
/// is true; see that function for the full definition of what makes a line
/// "next" — via ANSI bold escapes, or, if `no_markup` is true, by appending
/// `" <=="` to the plain-text line instead (no ANSI escapes at all).
/// `siblings` is the slice `node` was found in (empty for a root node,
/// which is never order-blocked). Sets `*found` when it marks a line so
/// only one line per render is ever highlighted.
fn node_line_content(
    node: NodeRef,
    siblings: &[parser::Subtask],
    identity: Option<(&ResolvedIdentity, &Config)>,
    no_markup: bool,
    depth: usize,
    found: &mut bool,
) -> String {
    let mut content = String::new();
    for _ in 0..depth {
        content.push_str("  ");
    }
    let status = node.status();
    let title = node.title();
    let is_next = !*found && rules::is_next_eligible_leaf(node, siblings, identity);
    if is_next {
        *found = true;
    }
    if is_next && !no_markup {
        content.push_str(crate::formatter::BOLD);
    }
    content.push_str(status_marker(status));
    content.push(' ');
    content.push_str(title);
    if is_next {
        if no_markup {
            content.push_str(" <==");
        } else {
            content.push_str(crate::formatter::RESET);
        }
    }
    content
}

/// Returns the textual checkbox for a [`parser::Status`]: `[ ]`, `[x]`, or `[-]`.
fn status_marker(status: &parser::Status) -> &'static str {
    match status {
        parser::Status::Todo => "[ ]",
        parser::Status::Done => "[x]",
        parser::Status::Cancelled => "[-]",
    }
}
