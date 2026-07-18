//! CLI for the `agile` binary.
//!
//! The library exposes this module so the binary can be a thin shim around
//! [`run`]. Subcommand-specific logic lives in [`subcommands`]; helpers shared
//! across subcommands live in [`common`].

use crate::config;
use clap::{Parser, Subcommand};
use log::error;
use std::path::Path;

pub mod common;
pub mod logger;
pub mod subcommands;

#[derive(Parser)]
#[command(
    name = "agile",
    about = "Plain-text, version-controlled task management",
    long_about = "\
Plain-text, version-controlled task management.

Reads all *.agile.md files found anywhere under the current directory.
Files are prioritised by their path relative to the project root, so
tasks/50_current/001.agile.md outranks tasks/60_backlog/001.agile.md.

Run without a subcommand to open the next task in $VISUAL / $EDITOR."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Task operations
    #[command(visible_alias = "tasks")]
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Show recognised task files in priority order
    ///
    /// Prints one line per file: filename followed by its path relative
    /// to the current directory. Files are sorted alphabetically by
    /// filename only — their directory path does not affect ordering.
    /// This sort order is the global task priority order across files.
    #[command(visible_alias = "files")]
    File {
        /// Show only the first COUNT files
        #[arg(short = 'n', long, value_name = "COUNT")]
        next: Option<usize>,

        /// Show only the last COUNT files
        #[arg(long, value_name = "COUNT")]
        last: Option<usize>,
    },

    /// Validate task files against the built-in rule set
    ///
    /// Parses every *.agile.md file under the current directory and reports
    /// each issue as `<path>:<line>: <message>` on stdout. Exits with status 1
    /// if any issue is found, 0 if the project is clean.
    Check {
        /// Override the identity used for the E013 assignment/completion
        /// check with a literal `[Users.X]` key, instead of resolving it
        /// from `git config user.email`/`user.name`. Useful in CI, where the
        /// runner's git identity isn't the PR author's. A value that doesn't
        /// match any configured user is always treated as unauthorized for
        /// assigned tasks (never silently skipped).
        #[arg(long, value_name = "USER")]
        r#as: Option<String>,

        /// Override the git ref used as the "old" (last-known-good) side of
        /// the E013 assignment/completion diff, instead of the hard-coded
        /// `HEAD`. The "new" side remains the working directory. Useful in
        /// CI, where the checked-out code is already fully committed (so
        /// working-copy-vs-HEAD would show no diff at all) — pass the PR's
        /// base branch/commit instead (e.g. `origin/main`).
        #[arg(long, value_name = "REF")]
        base: Option<String>,
    },

    /// ETA and velocity reporting
    When {
        /// Show the current velocity estimate (done-trend slope, weighted/day)
        #[arg(long)]
        velocity: bool,

        /// Show a terminal plot of total vs done weighted work over time.
        ///
        /// Requires `--next <rank>` to select the milestone boundary.
        #[arg(long, conflicts_with = "velocity")]
        plot: bool,

        /// Restrict velocity history to the last N days.
        ///
        /// Only valid with `--velocity`.
        #[arg(
            long,
            value_name = "DAYS",
            requires = "velocity",
            value_parser = clap::value_parser!(u32).range(1..)
        )]
        last: Option<u32>,

        /// Fit the vertical axis to the data range (default starts at zero).
        ///
        /// Only valid with `--plot`.
        #[arg(long, requires = "plot")]
        fit: bool,

        /// Select the Nth milestone rank.
        ///
        /// Currently used by `--plot` to choose the milestone boundary.
        #[arg(long, value_name = "RANK", conflicts_with = "velocity")]
        next: Option<usize>,
    },

    /// Show currently closed tasks with completion date when known
    History,
}

#[derive(Subcommand)]
pub enum TaskAction {
    /// List tasks in priority order
    ///
    /// Without arguments, prints every incomplete (`[ ]`) top-level task
    /// from all *.agile.md files in priority order, each with its full
    /// subtask tree. Each task is shown with its status marker ([ ] todo,
    /// [x] done, [-] cancelled) and subtasks indented by two spaces per
    /// level.
    List {
        /// A 1-based, inclusive range over the top-level tasks that would
        /// otherwise be shown (respecting `--all`/`--mine`), e.g. `2:4`
        /// shows the 2nd through 4th such tasks (each with its own
        /// subtree). Takes precedence over `--next`/`--last` if given.
        range: Option<String>,

        /// Show only the first COUNT entries
        #[arg(short = 'n', long, value_name = "COUNT")]
        next: Option<usize>,

        /// Show only the last COUNT entries
        #[arg(long, value_name = "COUNT")]
        last: Option<usize>,

        /// Show all tasks including done and cancelled
        #[arg(short = 'a', long)]
        all: bool,

        /// Only list top-level tasks that are unassigned or assigned to me
        /// (directly or via a group) — same eligibility rule as the E013
        /// assignment/completion check and `agile task next --mine`.
        #[arg(long)]
        mine: bool,

        /// Resolve `--mine` as this literal `[Users.X]` config key instead
        /// of the git identity from `git config user.email`/`user.name`.
        /// Implies `--mine` even when `--mine` isn't given.
        #[arg(long, value_name = "USER")]
        r#as: Option<String>,
    },

    /// Show the next highest-priority incomplete task(s)
    ///
    /// With no arguments, returns the first incomplete ([ ]) top-level task
    /// across all task files in priority order, including its full subtask
    /// tree. Skips done ([x]) and cancelled ([-]) tasks. Prints nothing if
    /// every task is complete or cancelled.
    #[command(visible_alias = "show")]
    Next {
        /// A plain count (e.g. `3`) to show the next N incomplete top-level
        /// tasks, or a dotted address (e.g. `1.2`, `2.1.4`) to show one
        /// specific (sub)task by position: the first number selects the Nth
        /// still-incomplete top-level task (in priority order); each
        /// subsequent number selects the Nth direct child of the
        /// previously-selected node (in document order, any status).
        /// Omit to show just the single next task.
        address: Option<String>,

        /// Only show task(s) that are unassigned (open to anyone) or
        /// assigned to me, directly or via a group — the same eligibility
        /// rule as the E013 assignment/completion check. Can be combined
        /// with a plain count (e.g. `next 3 --mine`), but not with a
        /// dotted address.
        #[arg(long)]
        mine: bool,

        /// Resolve `--mine` as this literal `[Users.X]` config key instead
        /// of the git identity from `git config user.email`/`user.name`.
        /// Implies `--mine` even when `--mine` isn't given.
        #[arg(long, value_name = "USER")]
        r#as: Option<String>,
    },

    /// Mark the (sub)task at ADDRESS done
    ///
    /// ADDRESS uses the same scheme as `agile task next`'s dotted address:
    /// e.g. `2` for the 2nd still-incomplete top-level task, or `1.3` for
    /// its 3rd direct child. Refuses (printing the violated rule instead of
    /// writing the file) if marking the node done would leave it with
    /// incomplete required children, missing required subtasks, or a
    /// disallowed cancelled required subtask. Only reads/writes the one
    /// file the addressed task lives in — it doesn't re-validate the rest
    /// of the project.
    Done {
        /// Dotted address, e.g. `2` or `1.3`.
        address: String,
    },

    /// Revert the (sub)task at ADDRESS back to todo
    ///
    /// The inverse of `agile task done`: always succeeds if the addressed
    /// node is currently done (`[x]`) — there are no completion rules to
    /// satisfy in reverse. ADDRESS uses *exactly* the same scheme as
    /// `agile task done`: `2` for the 2nd still-incomplete top-level task,
    /// or `1.3` for its 3rd direct child (any status). This is meant for
    /// correcting a mistakenly-completed subtask while its parent task is
    /// still open — a top-level task that is itself already fully done is
    /// consequently unreachable by this address, since only incomplete
    /// top-level tasks are counted; reopening a whole completed task isn't
    /// supported here (a dedicated command may be added for that later).
    Undone {
        /// Dotted address, e.g. `2` or `1.3`.
        address: String,
    },
}

/// Parses CLI arguments and dispatches to the matching subcommand.
///
/// This is the entry point used by `src/main.rs`. It is the only function the
/// binary needs to call.
pub fn run() {
    log::debug!("agile cli run()");

    let root = Path::new(".");
    let config = match config::Config::load(root) {
        Ok(c) => c,
        Err(e) => {
            error!("{e}");
            std::process::exit(1);
        }
    };

    match Cli::parse().command {
        None => subcommands::default::run(root),
        Some(Command::File { next, last }) => {
            subcommands::list::run_files(root, next, last);
        }
        Some(Command::Task {
            action:
                TaskAction::List {
                    range,
                    next,
                    last,
                    all,
                    mine,
                    r#as,
                },
        }) => {
            subcommands::list::run_tasks(
                root,
                &config,
                next,
                last,
                all,
                mine,
                r#as.as_deref(),
                range.as_deref(),
            );
        }
        Some(Command::Task {
            action:
                TaskAction::Next {
                    address,
                    mine,
                    r#as,
                },
        }) => {
            subcommands::task::run_next(root, &config, address.as_deref(), mine, r#as.as_deref());
        }
        Some(Command::Task {
            action: TaskAction::Done { address },
        }) => {
            subcommands::task::run_done(root, &config, &address);
        }
        Some(Command::Task {
            action: TaskAction::Undone { address },
        }) => {
            subcommands::task::run_undone(root, &config, &address);
        }
        Some(Command::Check { r#as, base }) => {
            subcommands::check::run(root, &config, r#as.as_deref(), base.as_deref());
        }
        Some(Command::When {
            velocity,
            plot,
            fit,
            last,
            next,
        }) => {
            subcommands::when::run(root, &config, next, velocity, plot, fit, last);
        }
        Some(Command::History) => {
            subcommands::history::run(root);
        }
    }
}

#[cfg(test)]
mod tests;
