use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(
    name = "agile",
    about = "Plain-text, version-controlled task management",
    long_about = "\
Plain-text, version-controlled task management.

Reads all *.agile.md files found anywhere under the current directory.
Files are prioritised alphabetically by filename (path is ignored), so
tasks in a_current.agile.md outrank tasks in b_backlog.agile.md.

Run without a subcommand to list all tasks.",
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
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

        #[command(subcommand)]
        what: Option<ListWhat>,
    },

    /// Task operations
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
}

#[derive(Subcommand)]
enum ListWhat {
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
    },
}

#[derive(Subcommand)]
enum TaskAction {
    /// Show the next highest-priority incomplete task
    ///
    /// Returns the first incomplete ([ ]) top-level task across all task
    /// files in priority order, including its full subtask tree. Skips
    /// done ([x]) and cancelled ([-]) tasks. Prints nothing if every
    /// task is complete or cancelled.
    Next,
}

fn main() {
    let root = Path::new(".");
    match Cli::parse().command.unwrap_or(Command::List { what: None, next: None }) {
        Command::List { what: None, next } => {
            let blocks = mdagile::list_task_blocks(&mdagile::read_task_files(root));
            let result: String = match next {
                Some(n) => blocks.into_iter().take(n).collect(),
                None    => blocks.into_iter().collect(),
            };
            print!("{result}");
        }
        Command::List { what: Some(ListWhat::Files { next }), .. } => {
            let paths = mdagile::find_task_files(root);
            let limited: Vec<_> = match next {
                Some(n) => paths.into_iter().take(n).collect(),
                None    => paths,
            };
            print!("{}", mdagile::format_file_list(&limited));
        }
        Command::Task { action: TaskAction::Next } => {
            print!("{}", mdagile::next_task(&mdagile::read_task_files(root)));
        }
    }
}
