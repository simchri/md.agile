//! CLI for the `agile` binary.
//!
//! The library exposes this module so the binary can be a thin shim around
//! [`run`]. Subcommand-specific logic lives in [`subcommands`]; helpers shared
//! across subcommands live in [`common`].

use clap::{Parser, Subcommand};
use std::path::Path;

pub mod common;
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
    /// List tasks or files (default: tasks)
    ///
    /// Without a subcommand, prints every task from all *.agile.md files
    /// in priority order. Each task is shown with its status marker
    /// ([ ] todo, [x] done, [-] cancelled) and subtasks indented by two
    /// spaces per level.
    List {
        /// Show only the first COUNT entries
        #[arg(short = 'n', long, value_name = "COUNT")]
        next: Option<usize>,

        /// Show only the last COUNT entries
        #[arg(long, value_name = "COUNT")]
        last: Option<usize>,

        /// Show all tasks including done and cancelled
        #[arg(short = 'a', long)]
        all: bool,

        #[command(subcommand)]
        what: Option<ListWhat>,
    },

    /// Task operations
    #[command(alias = "tasks")]
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Validate task files against the built-in rule set
    ///
    /// Parses every *.agile.md file under the current directory and reports
    /// each issue as `<path>:<line>: <message>` on stdout. Exits with status 1
    /// if any issue is found, 0 if the project is clean.
    Check,
}

#[derive(Subcommand)]
pub enum ListWhat {
    /// List all tasks (same as agile list with no subcommand)
    Tasks {
        /// Show only the first COUNT tasks
        #[arg(short = 'n', long, value_name = "COUNT")]
        next: Option<usize>,

        /// Show only the last COUNT tasks
        #[arg(long, value_name = "COUNT")]
        last: Option<usize>,

        /// Show all tasks including done and cancelled
        #[arg(short = 'a', long)]
        all: bool,
    },

    /// Show recognised task files in priority order
    ///
    /// Prints one line per file: filename followed by its path relative
    /// to the current directory. Files are sorted alphabetically by
    /// filename only — their directory path does not affect ordering.
    /// This sort order is the global task priority order across files.
    Files {
        /// Show only the first COUNT files
        #[arg(short = 'n', long, value_name = "COUNT")]
        next: Option<usize>,

        /// Show only the last COUNT files
        #[arg(long, value_name = "COUNT")]
        last: Option<usize>,
    },
}

#[derive(Subcommand)]
pub enum TaskAction {
    /// Show the next highest-priority incomplete task
    ///
    /// Returns the first incomplete ([ ]) top-level task across all task
    /// files in priority order, including its full subtask tree. Skips
    /// done ([x]) and cancelled ([-]) tasks. Prints nothing if every
    /// task is complete or cancelled.
    Next,
}

/// Parses CLI arguments and dispatches to the matching subcommand.
///
/// This is the entry point used by `src/main.rs`. It is the only function the
/// binary needs to call.
pub fn run() {
    let root = Path::new(".");
    match Cli::parse().command {
        None => subcommands::default::run(root),
        Some(Command::List {
            what: None,
            next,
            last,
            all,
        })
        | Some(Command::List {
            what: Some(ListWhat::Tasks { next, last, all }),
            ..
        }) => {
            subcommands::list::run_tasks(root, next, last, all);
        }
        Some(Command::List {
            what: Some(ListWhat::Files { next, last }),
            ..
        }) => {
            subcommands::list::run_files(root, next, last);
        }
        Some(Command::Task {
            action: TaskAction::Next,
        }) => {
            subcommands::task::run_next(root);
        }
        Some(Command::Check) => {
            subcommands::check::run(root);
        }
    }
}

#[cfg(test)]
mod tests;
