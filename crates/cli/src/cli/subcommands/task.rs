//! `agile task <action>` — task-centric subcommands.

use crate::checker;
use crate::cli::common::{find_task_files, parse_file, render_subtask_as_root, render_task};
use crate::config::Config;
use crate::formatter;
use crate::parser::{FileItem, Status, Subtask};
use crate::rules::{self, Issue, NodeRef, ResolvedIdentity};
use std::path::{Path, PathBuf};

/// `agile task next [ADDRESS]` entry point.
///
/// `ADDRESS` uses exactly the same resolution as `agile task done`'s
/// address (see [`resolve_address`]): a plain number (e.g. `3`) selects the
/// 3rd matching top-level task and prints *only* that one task — it does
/// not print tasks 1 through 3. A dotted address (e.g. `1.2`, `2.1.4`)
/// descends into direct children (any status) from there, to arbitrary
/// depth, and prints that one (sub)task as its own root, subtree included.
/// With no address at all, this defaults to address `1` (the single next
/// incomplete top-level task) — but unlike an explicit address, finding no
/// match here is not an error: it just prints nothing (there may simply be
/// no incomplete tasks left).
///
/// `mine` restricts the top-level tasks counted by the address's first
/// segment to ones that are unassigned or assigned to the resolved identity
/// (`as_user`, or the git identity if `as_user` is `None`) — see
/// [`rules::is_eligible_for`]. `mine` is only valid with no address or a
/// plain number — combining it with a dotted address is a hard error, since
/// a dotted address already names one exact node regardless of assignment.
/// `as_user` implies `mine` even if `mine` itself is `false`, so `--as alice`
/// alone (without `--mine`) still filters by alice's eligibility.
pub fn run_next(
    root: &Path,
    config: &Config,
    address: Option<&str>,
    mine: bool,
    as_user: Option<&str>,
) {
    let mine = mine || as_user.is_some();

    let parts = match address.map(parse_address) {
        None => None,
        Some(Some(parts)) => Some(parts),
        Some(None) => {
            log::error!(
                "invalid task address {:?} — expected a number or dotted address like `1.2`",
                address.unwrap()
            );
            std::process::exit(1);
        }
    };

    let dotted = matches!(&parts, Some(p) if p.len() > 1);

    if mine && dotted {
        log::error!(
            "`--mine` cannot be combined with a dotted address (a dotted address already names one exact task)"
        );
        std::process::exit(1);
    }

    let identity = if mine {
        match checker::resolve_cli_identity(root, config, as_user) {
            Ok(identity) => Some(identity),
            Err(e) => {
                log::error!("{e}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let explicit_address = parts.is_some();
    let resolve_parts = parts.unwrap_or_else(|| vec![1]);

    match resolve_address(
        root,
        &resolve_parts,
        config,
        identity.as_ref(),
        Status::Todo,
    ) {
        Ok(resolved) => print!("{}", render_resolved(&resolved)),
        Err(e) => {
            if explicit_address {
                log::error!("{e}");
                std::process::exit(1);
            }
            // No address was given at all: there simply being no matching
            // task right now (e.g. everything done/cancelled) is not an
            // error condition, so print nothing and exit 0.
        }
    }
}

/// `agile task done ADDRESS` entry point.
///
/// Resolves `address` (see [`parse_address`]) to a single (sub)task, checks
/// that marking it done wouldn't violate the "incomplete children" (E004),
/// "missing required subtasks" (E010), or "cancelled required subtask not
/// allowed" (E012) rules, and — only if clean — flips its status box to
/// `[x]` in place in its own source file. Prints the violated issue(s) and
/// exits 1 instead of writing anything if the node isn't actually
/// completable yet, or if it isn't a todo task to begin with.
pub fn run_done(root: &Path, config: &Config, address: &str) {
    let parts = match parse_address(address) {
        Some(parts) => parts,
        None => {
            log::error!(
                "invalid task address {address:?} — expected a number or dotted address like `1.2`"
            );
            std::process::exit(1);
        }
    };

    let resolved = match resolve_address(root, &parts, config, None, Status::Todo) {
        Ok(resolved) => resolved,
        Err(e) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    };

    let line = resolved.node_ref().location().line;
    match mark_node_done(&resolved.file, &resolved.items, line, config) {
        Ok(title) => println!("done: {title}"),
        Err(MarkDoneError::NotTodo(title)) => {
            log::error!("task {address:?} ({title}) is not a todo task");
            std::process::exit(1);
        }
        Err(MarkDoneError::RuleViolations(issues)) => {
            for issue in &issues {
                print!("{}", formatter::format_issue(issue));
            }
            std::process::exit(1);
        }
        Err(MarkDoneError::NotFound) => {
            // Can't happen: `line` was just read from `resolved`'s own parsed
            // `items`, so a node is guaranteed to start there.
            unreachable!("resolved address line vanished from its own parsed items");
        }
        Err(MarkDoneError::Io(e)) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    }
}

/// `agile task undone ADDRESS` entry point.
///
/// Reverts the (sub)task at `address` back to todo (`[ ]`). Unlike
/// `agile task done`, there are no completion rules to satisfy in reverse —
/// a done task can always be un-done regardless of its parent's or
/// children's state.
///
/// `address` uses *exactly* the same resolution as `agile task done`'s
/// address (see [`resolve_address`]): the first segment selects the Nth
/// still-incomplete top-level task, and every subsequent segment selects
/// the Nth direct child (any status) from there. This is intentional: this
/// command exists to correct a mistakenly-completed *subtask* while its
/// parent is still open, not to reopen an already fully-completed
/// top-level task — a top-level task that's itself done is consequently
/// unreachable by this address (there's no number that resolves to it),
/// which is an accepted limitation, not a bug. Reopening a whole completed
/// top-level task is expected to need a different, dedicated command if
/// it's ever added.
pub fn run_undone(root: &Path, config: &Config, address: &str) {
    let parts = match parse_address(address) {
        Some(parts) => parts,
        None => {
            log::error!(
                "invalid task address {address:?} — expected a number or dotted address like `1.2`"
            );
            std::process::exit(1);
        }
    };

    let resolved = match resolve_address(root, &parts, config, None, Status::Todo) {
        Ok(resolved) => resolved,
        Err(e) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    };

    let line = resolved.node_ref().location().line;
    match mark_node_undone(&resolved.file, &resolved.items, line) {
        Ok(title) => println!("undone: {title}"),
        Err(MarkUndoneError::NotDone(title)) => {
            log::error!("task {address:?} ({title}) is not a done task");
            std::process::exit(1);
        }
        Err(MarkUndoneError::NotFound) => {
            // Can't happen: `line` was just read from `resolved`'s own parsed
            // `items`, so a node is guaranteed to start there.
            unreachable!("resolved address line vanished from its own parsed items");
        }
        Err(MarkUndoneError::Io(e)) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    }
}

/// Returns the first incomplete top-level task block from `items`.
///
/// Scans tasks in document order and returns the rendered subtree of the first
/// task whose top-level marker is todo (`[ ]`). Done and cancelled tasks are
/// skipped. Returns an empty string if every task is complete or cancelled, or
/// if there are no tasks.
pub fn next_task(items: &[FileItem]) -> String {
    next_n_tasks(items, 1, None, &Config::default())
}

/// Returns the rendered blocks of the first `n` incomplete top-level tasks in
/// `items`, in document order. If `identity` is `Some`, tasks assigned to
/// someone else (and not also unassigned) are skipped — see
/// [`rules::is_eligible_for`]. Returns fewer than `n` blocks (possibly none)
/// if there aren't enough matching tasks.
fn next_n_tasks(
    items: &[FileItem],
    n: usize,
    identity: Option<&ResolvedIdentity>,
    config: &Config,
) -> String {
    let mut out = String::new();
    let mut found = 0;
    for item in items {
        if let FileItem::Task(task) = item {
            if task.status != Status::Todo {
                continue;
            }
            if let Some(identity) = identity {
                if !rules::is_eligible_for(NodeRef::Task(task), identity, config) {
                    continue;
                }
            }
            render_task(task, &mut out);
            found += 1;
            if found >= n {
                break;
            }
        }
    }
    out
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

/// Parses a task address like `"2"` or `"1.3.2"` into its 1-based numeric
/// segments. Returns `None` if `s` is empty, has an empty segment (e.g.
/// `"1."` or `"1..2"`), or any segment isn't a positive integer (`"0"` and
/// negative/non-numeric segments are both invalid — addressing is 1-based).
pub(crate) fn parse_address(s: &str) -> Option<Vec<usize>> {
    let mut result = Vec::new();
    for part in s.split('.') {
        if part.is_empty() {
            return None;
        }
        match part.parse::<usize>() {
            Ok(0) => return None,
            Ok(n) => result.push(n),
            Err(_) => return None,
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// A task address resolved to a concrete (sub)task within one specific file.
///
/// Holds the full parsed contents of that one file (`items`) plus enough
/// indices to navigate straight to the addressed node via [`node_ref`],
/// without re-parsing or re-scanning anything.
///
/// [`node_ref`]: ResolvedAddress::node_ref
pub(crate) struct ResolvedAddress {
    file: PathBuf,
    items: Vec<FileItem>,
    task_index: usize,
    child_indices: Vec<usize>,
}

impl ResolvedAddress {
    /// Returns a [`NodeRef`] pointing at the addressed (sub)task.
    pub(crate) fn node_ref(&self) -> NodeRef<'_> {
        let task = match &self.items[self.task_index] {
            FileItem::Task(t) => t,
            _ => unreachable!("task_index always points at a FileItem::Task"),
        };
        let mut node = NodeRef::Task(task);
        for &idx in &self.child_indices {
            let children = node.children();
            node = NodeRef::Subtask(&children[idx]);
        }
        node
    }
}

/// Resolves a parsed address (see [`parse_address`]) to a concrete
/// (sub)task.
///
/// `parts[0]` selects the Nth top-level task whose status is
/// `target_status` (1-based, across all task files in priority order). Each
/// subsequent `parts[i]` selects the Nth direct child (1-based, document
/// order, any status) of the node selected by the previous segment.
///
/// If `eligible_for` is `Some`, top-level candidates are further restricted
/// to ones [`rules::is_eligible_for`] that identity (unassigned, or assigned
/// to them directly or via a group) — this is what backs
/// `agile task next N --mine`. `agile task done`/`agile task undone` never
/// pass an identity, since an address there always names one exact task
/// regardless of who it's assigned to.
///
/// Files are parsed one at a time and scanning stops as soon as the
/// addressed top-level task is found — later files are never even read —
/// so this stays cheap regardless of how many task files a project has.
pub(crate) fn resolve_address(
    root: &Path,
    parts: &[usize],
    config: &Config,
    eligible_for: Option<&ResolvedIdentity>,
    target_status: Status,
) -> Result<ResolvedAddress, String> {
    let Some((&first, rest)) = parts.split_first() else {
        return Err("empty task address".to_string());
    };

    let mut matching_count = 0usize;
    for file in find_task_files(root) {
        let items = parse_file(&file);
        for (idx, item) in items.iter().enumerate() {
            let FileItem::Task(task) = item else {
                continue;
            };
            if task.status != target_status {
                continue;
            }
            if let Some(identity) = eligible_for {
                if !rules::is_eligible_for(NodeRef::Task(task), identity, config) {
                    continue;
                }
            }
            matching_count += 1;
            if matching_count != first {
                continue;
            }

            let mut children: &[Subtask] = &task.children;
            let mut child_indices = Vec::with_capacity(rest.len());
            for &part in rest {
                if part > children.len() {
                    return Err(format!(
                        "no such task: address {} has no child #{part} at that level (only {} there)",
                        format_address(parts),
                        children.len()
                    ));
                }
                child_indices.push(part - 1);
                children = &children[part - 1].children;
            }
            return Ok(ResolvedAddress {
                file,
                items,
                task_index: idx,
                child_indices,
            });
        }
    }
    let status_word = match target_status {
        Status::Todo => "incomplete",
        Status::Done => "done",
        Status::Cancelled => "cancelled",
    };
    Err(format!(
        "no such task: address {} starts at {status_word} top-level task #{first}, but only {matching_count} matching {status_word} top-level task(s) exist",
        format_address(parts)
    ))
}

fn format_address(parts: &[usize]) -> String {
    parts
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Renders the (sub)task resolved by `resolved` as its own root block,
/// exactly like [`render_task`] would for a top-level task.
fn render_resolved(resolved: &ResolvedAddress) -> String {
    let mut out = String::new();
    match resolved.node_ref() {
        NodeRef::Task(task) => render_task(task, &mut out),
        NodeRef::Subtask(sub) => render_subtask_as_root(sub, &mut out),
    }
    out
}

/// Returns `line` with the status character inside its `[...]` box replaced
/// by `x`, or `None` if `indent` puts the box position past the end of
/// `line` (i.e. `line` isn't actually a task/subtask line at that indent).
///
/// The box's status character always sits at 0-based index `indent + 3`
/// (`"- ["` is 3 characters: `-`, ` `, `[`) — the character right after the
/// opening bracket.
pub(crate) fn set_status_done(line: &str, indent: usize) -> Option<String> {
    let pos = indent + 3;
    let mut chars: Vec<char> = line.chars().collect();
    if pos >= chars.len() {
        return None;
    }
    chars[pos] = 'x';
    Some(chars.into_iter().collect())
}

/// Returns `line` with the status character inside its `[...]` box replaced
/// by a space (todo), or `None` if `indent` puts the box position past the
/// end of `line` — the inverse of [`set_status_done`], used by
/// [`mark_node_undone`].
pub(crate) fn set_status_todo(line: &str, indent: usize) -> Option<String> {
    let pos = indent + 3;
    let mut chars: Vec<char> = line.chars().collect();
    if pos >= chars.len() {
        return None;
    }
    chars[pos] = ' ';
    Some(chars.into_iter().collect())
}

/// The reason [`mark_node_done`] refused to mark a (sub)task done.
#[derive(Debug, PartialEq)]
pub enum MarkDoneError {
    /// No task or subtask starts at the given line — e.g. a stale (file,
    /// line) identity captured before the file changed.
    NotFound,
    /// The addressed node exists but isn't a todo (`[ ]`) task; carries its
    /// title.
    NotTodo(String),
    /// The addressed node is a todo task but fails one or more completion
    /// rules (e.g. incomplete required children) — see [`rules::check_completable`].
    RuleViolations(Vec<Issue>),
    /// Reading or writing the file failed.
    Io(String),
}

/// Marks the (sub)task starting at `line` in `file` done (`[x]`), after
/// verifying it's a todo task that satisfies every completion rule (see
/// [`rules::check_completable`]) — the same checks `agile task done`
/// enforces via an address. Returns the node's title on success.
///
/// `items` must already be the parsed contents of `file` (the caller is
/// responsible for parsing — this function neither reads nor re-parses the
/// file except to perform the actual write). This lets a caller that has
/// already parsed the file for another purpose (e.g. [`resolve_address`])
/// reuse that work, and lets a caller that only knows a (file, line) pair —
/// e.g. the GUI, from a `TaskView` returned by an earlier listing — locate
/// the node itself via [`rules::find_node_by_line`] without needing to know
/// its position in the tree.
pub fn mark_node_done(
    file: &Path,
    items: &[FileItem],
    line: usize,
    config: &Config,
) -> Result<String, MarkDoneError> {
    let node = rules::find_node_by_line(items, line).ok_or(MarkDoneError::NotFound)?;

    if *node.status() != Status::Todo {
        return Err(MarkDoneError::NotTodo(node.title().to_string()));
    }

    let issues = rules::check_completable(node, config);
    if !issues.is_empty() {
        return Err(MarkDoneError::RuleViolations(issues));
    }

    let title = node.title().to_string();
    let indent = node.indent();
    write_done_line(file, line, indent).map_err(MarkDoneError::Io)?;
    Ok(title)
}

/// Rewrites one line of `file` in place to mark it done (`[x]`), preserving
/// every other line and the file's trailing-newline presence exactly.
fn write_done_line(file: &Path, line_no: usize, indent: usize) -> Result<(), String> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("could not read {}: {e}", file.display()))?;
    let had_trailing_newline = content.ends_with('\n');
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();

    if line_no == 0 || line_no > lines.len() {
        return Err(format!("line {line_no} out of range in {}", file.display()));
    }
    let new_line = set_status_done(&lines[line_no - 1], indent)
        .ok_or_else(|| format!("could not locate task box on {}:{line_no}", file.display()))?;
    lines[line_no - 1] = new_line;

    let mut new_content = lines.join("\n");
    if had_trailing_newline {
        new_content.push('\n');
    }
    std::fs::write(file, new_content)
        .map_err(|e| format!("could not write {}: {e}", file.display()))?;
    Ok(())
}

/// The reason [`mark_node_undone`] refused to revert a (sub)task to todo.
#[derive(Debug, PartialEq)]
pub enum MarkUndoneError {
    /// No task or subtask starts at the given line — e.g. a stale (file,
    /// line) identity captured before the file changed.
    NotFound,
    /// The addressed node exists but isn't a done (`[x]`) task; carries its
    /// title. This also covers cancelled (`[-]`) tasks — undone only
    /// reverts done tasks, not cancelled ones.
    NotDone(String),
    /// Reading or writing the file failed.
    Io(String),
}

/// Reverts the (sub)task starting at `line` in `file` to todo (`[ ]`),
/// after verifying it's currently a done task. Unlike [`mark_node_done`],
/// there are no completion rules to satisfy in reverse — a done task can
/// always be un-done regardless of its parent's or children's state.
/// Returns the node's title on success.
///
/// `items` must already be the parsed contents of `file` — see
/// [`mark_node_done`]'s docs for why the caller is responsible for parsing.
pub fn mark_node_undone(
    file: &Path,
    items: &[FileItem],
    line: usize,
) -> Result<String, MarkUndoneError> {
    let node = rules::find_node_by_line(items, line).ok_or(MarkUndoneError::NotFound)?;

    if *node.status() != Status::Done {
        return Err(MarkUndoneError::NotDone(node.title().to_string()));
    }

    let title = node.title().to_string();
    let indent = node.indent();
    write_todo_line(file, line, indent).map_err(MarkUndoneError::Io)?;
    Ok(title)
}

/// Rewrites one line of `file` in place to revert it to todo (`[ ]`),
/// preserving every other line and the file's trailing-newline presence
/// exactly — the inverse of [`write_done_line`].
fn write_todo_line(file: &Path, line_no: usize, indent: usize) -> Result<(), String> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("could not read {}: {e}", file.display()))?;
    let had_trailing_newline = content.ends_with('\n');
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();

    if line_no == 0 || line_no > lines.len() {
        return Err(format!("line {line_no} out of range in {}", file.display()));
    }
    let new_line = set_status_todo(&lines[line_no - 1], indent)
        .ok_or_else(|| format!("could not locate task box on {}:{line_no}", file.display()))?;
    lines[line_no - 1] = new_line;

    let mut new_content = lines.join("\n");
    if had_trailing_newline {
        new_content.push('\n');
    }
    std::fs::write(file, new_content)
        .map_err(|e| format!("could not write {}: {e}", file.display()))?;
    Ok(())
}

#[cfg(test)]
#[path = "task_tests.rs"]
mod tests;
