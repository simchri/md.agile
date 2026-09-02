//! `agile task <action>` — task-centric subcommands.

use crate::checker;
use crate::cli::common::{
    find_task_files, parse_file, render_subtask_as_root_highlighting_next_leaf,
    render_subtask_as_root_highlighting_previous_leaf, render_task_highlighting_next_leaf,
    render_task_highlighting_previous_leaf,
};
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
///
/// `full` additionally prints each (sub)task's body lines alongside its
/// title line.
/// `full` additionally prints each (sub)task's body lines alongside its
/// title line.
///
/// `no_markup` disables ANSI bold/color escapes entirely: the concrete next
/// actionable line is instead marked by appending `" <=="` to it. Useful
/// for scripting/piping output where ANSI escapes would be unwanted noise.
pub fn run_next(
    root: &Path,
    config: &Config,
    address: Option<&str>,
    mine: bool,
    as_user: Option<&str>,
    full: bool,
    no_markup: bool,
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
        TopLevelFilter::Status(Status::Todo),
    ) {
        Ok(resolved) => {
            let displayed_rank = resolved.rank_address(&resolve_parts[1..]);
            print!(
                "{}",
                render_resolved(
                    &resolved,
                    full,
                    identity.as_ref(),
                    config,
                    no_markup,
                    &displayed_rank,
                )
            )
        }
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

/// `agile task previous [ADDRESS]` entry point.
///
/// The mirror image of `agile task next`, walking *closed* top-level tasks
/// instead: `ADDRESS`'s first segment selects the Nth top-level task
/// counting back from the most recently touched one (see
/// [`TopLevelFilter::ClosedWorkReversed`]) — a top-level task counts the
/// moment it or any descendant is `Done`/`Cancelled`, so a partially
/// completed task counts too, not just a fully done one. Every subsequent
/// dotted segment descends into direct children exactly like `agile task
/// next`/`done`. With no address at all, this defaults to address `1` (the
/// single most recently touched top-level task) — and, like `agile task
/// next`, finding no match here (nothing closed at all yet) is not an
/// error: it just prints nothing.
///
/// The printed subtree is always the *whole* addressed top-level task (or
/// subtask), full tree descent, with dotted addresses computed in normal
/// forward document order from there — exactly like `agile task next`'s
/// output shape — except the highlighted line is the *last* node in
/// document order that [`rules::is_previous_task`] (the most recently
/// completed concrete unit of work), not the first actionable one.
///
/// `full` additionally prints each (sub)task's body lines. `no_markup`
/// disables ANSI bold/color escapes, marking the highlighted line with
/// `" <=="` instead.
pub fn run_previous(
    root: &Path,
    config: &Config,
    address: Option<&str>,
    full: bool,
    no_markup: bool,
) {
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

    let explicit_address = parts.is_some();
    let resolve_parts = parts.unwrap_or_else(|| vec![1]);

    match resolve_address(
        root,
        &resolve_parts,
        config,
        None,
        TopLevelFilter::ClosedWorkReversed,
    ) {
        Ok(resolved) => print!(
            "{}",
            render_resolved_previous(&resolved, full, no_markup, &format_address(&resolve_parts))
        ),
        Err(e) => {
            if explicit_address {
                log::error!("{e}");
                std::process::exit(1);
            }
            // No address was given at all: there simply being no closed
            // task yet is not an error condition, so print nothing and exit
            // 0, mirroring `agile task next`'s behavior.
        }
    }
}

/// `agile task done ADDRESS` entry point.
///
/// Resolves `address` (see [`parse_address`]) to a single (sub)task, checks
/// that marking it done wouldn't violate the "incomplete children" (E004),
/// "missing required subtasks" (E010), "cancelled required subtask not
/// allowed" (E012), or "unauthorized completion" (E013) rules, and — only if
/// clean — flips its status box to `[x]` in place in its own source file.
/// Prints the violated issue(s) and exits 1 instead of writing anything if
/// the node isn't actually completable yet, or if it isn't a todo task to
/// begin with.
///
/// The acting identity for the E013 check is resolved via
/// [`checker::resolve_task_done_identity`]: `as_user` (from `--as`) if
/// given, otherwise the live git identity of `root`. Unlike `--mine`, an
/// unresolvable identity (not a git repo, or no git identity configured)
/// never aborts the command outright — it's simply treated as unauthorized
/// for any *assigned* task, exactly like a git identity that doesn't match
/// any `[Users.X]` entry. Unassigned tasks are unaffected either way.
pub fn run_done(root: &Path, config: &Config, address: &str, as_user: Option<&str>) {
    let parts = match parse_address(address) {
        Some(parts) => parts,
        None => {
            log::error!(
                "invalid task address {address:?} — expected a number or dotted address like `1.2`"
            );
            std::process::exit(1);
        }
    };

    let resolved = match resolve_address(
        root,
        &parts,
        config,
        None,
        TopLevelFilter::Status(Status::Todo),
    ) {
        Ok(resolved) => resolved,
        Err(e) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    };

    let identity = checker::resolve_task_done_identity(root, config, as_user);
    let line = resolved.node_ref().location().line;
    match mark_node_done(&resolved.file, &resolved.items, line, config, &identity) {
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
/// `address` uses the same "reverse rank" resolution as `agile task
/// previous` (see [`TopLevelFilter::ClosedWorkReversed`]): the first segment
/// selects the Nth top-level task counting back from the last one with any
/// closed (`Done`/`Cancelled`) work in it — so a whole already fully-done
/// top-level task is reachable this way (as address `1` if it's the most
/// recently touched one), not just a still-open subtask of an otherwise
/// incomplete parent. Every subsequent segment selects the Nth direct child
/// (any status) from there, exactly like `agile task done`.
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

    let resolved = match resolve_address(
        root,
        &parts,
        config,
        None,
        TopLevelFilter::ClosedWorkReversed,
    ) {
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
    next_n_tasks(items, 1, None, &Config::default(), false)
}

/// Returns the rendered blocks of the first `n` incomplete top-level tasks in
/// `items`, in document order. If `identity` is `Some`, tasks assigned to
/// someone else (and not also unassigned) are skipped — see
/// [`rules::is_eligible_for`]. Returns fewer than `n` blocks (possibly none)
/// if there aren't enough matching tasks. `include_body` also prints each
/// node's body lines (see [`render_task_highlighting_next_leaf`]).
fn next_n_tasks(
    items: &[FileItem],
    n: usize,
    identity: Option<&ResolvedIdentity>,
    config: &Config,
    include_body: bool,
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
            let number = (found + 1).to_string();
            render_task_highlighting_next_leaf(
                task,
                include_body,
                identity.map(|identity| (identity, config)),
                false,
                &number,
                &mut out,
            );
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
    /// The resolved node's own **Rank** (its Task Address) — its 1-based
    /// position counting *every* top-level task matching [`TopLevelFilter`]
    /// (`Status`/`ClosedWorkReversed`), regardless of any `eligible_for`
    /// identity filter. This is deliberately *not* the same as the `first`
    /// segment passed in to [`resolve_address`] when an identity filter is
    /// active: `first` there instead counts only *eligible* candidates
    /// (`--mine`'s "Nth task I could work on"), so the two numbers diverge
    /// whenever an ineligible task is skipped along the way. Callers that
    /// print or otherwise expose a task's address (e.g. `agile task next`)
    /// must use this field, not `first` — it's the one number that always
    /// names the same task regardless of `--mine`/`--as`, matching what
    /// `agile task done <N>` (never identity-filtered) actually consumes.
    /// See the "Rank" glossary entry.
    top_level_rank: usize,
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

    /// Returns the full dotted rank/address for the resolved node —
    /// [`Self::top_level_rank`] as the first segment, followed by the
    /// same child segments the caller originally requested (children are
    /// never identity-filtered, so those never diverge from what was
    /// asked for).
    pub(crate) fn rank_address(&self, requested_child_parts: &[usize]) -> String {
        format_address(
            std::iter::once(self.top_level_rank)
                .chain(requested_child_parts.iter().copied())
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }
}

/// The rule deciding which top-level tasks count as candidates for a
/// resolved address's first segment, and in which order they're numbered.
/// See [`resolve_address`].
#[derive(Clone, Copy)]
pub(crate) enum TopLevelFilter {
    /// A top-level task counts iff its own status equals the given
    /// [`Status`], numbered in normal document/priority order (address `1`
    /// is the *first* matching task). Used by `agile task next`/`done`/the
    /// original `agile task undone` behavior (all pass [`Status::Todo`]).
    Status(Status),
    /// A top-level task counts iff [`rules::has_closed_work`] holds for it
    /// (itself or any descendant is `Done`/`Cancelled`), numbered in
    /// *reverse* document/priority order — address `1` is the *last*
    /// matching task. This is the "reverse rank" candidacy used by
    /// `agile task previous` and the generalized `agile task undone`.
    ClosedWorkReversed,
}

/// Resolves a parsed address (see [`parse_address`]) to a concrete
/// (sub)task.
///
/// `parts[0]` selects the Nth top-level task that counts as a candidate per
/// `filter` (1-based, across all task files, in the order `filter`
/// prescribes). Each subsequent `parts[i]` selects the Nth direct child
/// (1-based, document order, any status) of the node selected by the
/// previous segment.
///
/// If `eligible_for` is `Some`, top-level candidates are further restricted
/// to ones [`rules::is_eligible_for`] that identity (unassigned, or assigned
/// to them directly or via a group) — this is what backs
/// `agile task next N --mine`. `agile task done`/`agile task undone` never
/// pass an identity, since an address there always names one exact task
/// regardless of who it's assigned to.
///
/// For [`TopLevelFilter::Status`], files are parsed one at a time and
/// scanning stops as soon as the addressed top-level task is found — later
/// files are never even read — so this stays cheap regardless of how many
/// task files a project has. [`TopLevelFilter::ClosedWorkReversed`] needs
/// every candidate up front to number them in reverse, so it parses all
/// task files unconditionally.
pub(crate) fn resolve_address(
    root: &Path,
    parts: &[usize],
    config: &Config,
    eligible_for: Option<&ResolvedIdentity>,
    filter: TopLevelFilter,
) -> Result<ResolvedAddress, String> {
    let Some((&first, rest)) = parts.split_first() else {
        return Err("empty task address".to_string());
    };

    let matches_filter = |task: &crate::parser::Task| -> bool {
        match filter {
            TopLevelFilter::Status(target_status) => task.status == target_status,
            TopLevelFilter::ClosedWorkReversed => rules::has_closed_work(NodeRef::Task(task)),
        }
    };

    let finish = |file: PathBuf,
                  items: Vec<FileItem>,
                  idx: usize,
                  rest: &[usize],
                  top_level_rank: usize|
     -> Result<ResolvedAddress, String> {
        let task = match &items[idx] {
            FileItem::Task(t) => t,
            _ => unreachable!("idx always points at a FileItem::Task"),
        };
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
        Ok(ResolvedAddress {
            file,
            items,
            task_index: idx,
            child_indices,
            top_level_rank,
        })
    };

    let is_eligible = |task: &crate::parser::Task| -> bool {
        eligible_for
            .is_none_or(|identity| rules::is_eligible_for(NodeRef::Task(task), identity, config))
    };

    match filter {
        TopLevelFilter::Status(target_status) => {
            // `rank` counts every top-level task matching `target_status`,
            // regardless of eligibility — this is the task's stable rank
            // (see `ResolvedAddress::top_level_rank`), unaffected by
            // `--mine`/`--as`. `eligible_seen` counts only the ones
            // `is_eligible` accepts, and is what `first` (the parsed
            // address) actually walks: `agile task next N --mine` selects
            // the Nth *eligible* task, but still reports that task's
            // overall rank once found.
            let mut rank = 0usize;
            let mut eligible_seen = 0usize;
            for file in find_task_files(root) {
                let items = parse_file(&file);
                for (idx, item) in items.iter().enumerate() {
                    let FileItem::Task(task) = item else {
                        continue;
                    };
                    if !matches_filter(task) {
                        continue;
                    }
                    rank += 1;
                    if !is_eligible(task) {
                        continue;
                    }
                    eligible_seen += 1;
                    if eligible_seen != first {
                        continue;
                    }
                    return finish(file, items, idx, rest, rank);
                }
            }
            let status_word = match target_status {
                Status::Todo => "incomplete",
                Status::Done => "done",
                Status::Cancelled => "cancelled",
            };
            Err(format!(
                "no such task: address {} starts at {status_word} top-level task #{first}, but only {eligible_seen} matching {status_word} top-level task(s) exist",
                format_address(parts)
            ))
        }
        TopLevelFilter::ClosedWorkReversed => {
            // Collected in forward document order first so each candidate's
            // reverse rank (its reverse position counting *all* matches,
            // regardless of eligibility) can be computed from `total`/its
            // forward index below, before eligibility narrows down which
            // ones `first` can actually select.
            let mut candidates: Vec<(PathBuf, Vec<FileItem>, usize, bool)> = Vec::new();
            for file in find_task_files(root) {
                let items = parse_file(&file);
                for (idx, item) in items.iter().enumerate() {
                    let FileItem::Task(task) = item else {
                        continue;
                    };
                    if !matches_filter(task) {
                        continue;
                    }
                    candidates.push((file.clone(), items.clone(), idx, is_eligible(task)));
                }
            }
            let total = candidates.len();
            let eligible_total = candidates.iter().filter(|(.., eligible)| *eligible).count();
            if first > eligible_total || first == 0 {
                return Err(format!(
                    "no such task: address {} starts at closed top-level task #{first}, but only {eligible_total} matching closed top-level task(s) exist",
                    format_address(parts)
                ));
            }
            // Walk candidates in reverse document order (address `1` is the
            // most recently touched one), counting off `first` eligible
            // ones; the reverse rank is the candidate's own reverse
            // position among *all* candidates, eligible or not.
            let mut eligible_seen = 0usize;
            for (forward_idx, (file, items, idx, eligible)) in
                candidates.into_iter().enumerate().rev()
            {
                if !eligible {
                    continue;
                }
                eligible_seen += 1;
                if eligible_seen != first {
                    continue;
                }
                let reverse_rank = total - forward_idx;
                return finish(file, items, idx, rest, reverse_rank);
            }
            unreachable!("first <= eligible_total was already checked above");
        }
    }
}

fn format_address(parts: &[usize]) -> String {
    parts
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Renders the (sub)task resolved by `resolved` as its own root block,
/// exactly like [`render_task`] would for a top-level task, but marks the
/// first `Todo` leaf in the subtree — the concrete next actionable task —
/// and, if `full` is true, also prints body lines (see
/// [`render_task_highlighting_next_leaf`]). If `identity` is `Some`, only a
/// leaf eligible for that identity is marked (see
/// [`rules::is_eligible_for`]); otherwise the first `Todo` leaf is marked
/// unconditionally. Normally the marked line is bolded (ANSI escapes); if
/// `no_markup` is true, it's instead marked by appending `" <=="` in
/// plain text. `root_number` is the dotted address that was resolved to
/// reach `resolved` (e.g. `"1"` for the default address, `"1.2"` for an
/// explicit dotted address) — printed as the root's own number, with its
/// descendants numbered from there (see
/// [`render_task_highlighting_next_leaf`]).
///
/// [`render_task`]: crate::cli::common::render_task
fn render_resolved(
    resolved: &ResolvedAddress,
    full: bool,
    identity: Option<&ResolvedIdentity>,
    config: &Config,
    no_markup: bool,
    root_number: &str,
) -> String {
    let mut out = String::new();
    let identity = identity.map(|identity| (identity, config));
    match resolved.node_ref() {
        NodeRef::Task(task) => render_task_highlighting_next_leaf(
            task,
            full,
            identity,
            no_markup,
            root_number,
            &mut out,
        ),
        NodeRef::Subtask(sub) => render_subtask_as_root_highlighting_next_leaf(
            sub,
            full,
            identity,
            no_markup,
            root_number,
            &mut out,
        ),
    }
    out
}

/// Same as [`render_resolved`], but for `agile task previous` — highlights
/// the last [`rules::is_previous_task`] node instead of the first
/// [`rules::is_next_task`] one, and never filters by identity (see
/// [`crate::cli::common::render_task_highlighting_previous_leaf`]).
fn render_resolved_previous(
    resolved: &ResolvedAddress,
    full: bool,
    no_markup: bool,
    root_number: &str,
) -> String {
    let mut out = String::new();
    match resolved.node_ref() {
        NodeRef::Task(task) => {
            render_task_highlighting_previous_leaf(task, full, no_markup, root_number, &mut out)
        }
        NodeRef::Subtask(sub) => render_subtask_as_root_highlighting_previous_leaf(
            sub,
            full,
            no_markup,
            root_number,
            &mut out,
        ),
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
/// enforces via an address, including the E013 "unauthorized completion"
/// check against `identity`. Returns the node's title on success.
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
    identity: &ResolvedIdentity,
) -> Result<String, MarkDoneError> {
    let node = rules::find_node_by_line(items, line).ok_or(MarkDoneError::NotFound)?;

    if *node.status() != Status::Todo {
        return Err(MarkDoneError::NotTodo(node.title().to_string()));
    }

    let issues = rules::check_completable(items, node, config, identity);
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
