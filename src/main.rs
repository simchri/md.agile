use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(
    name = "agile",
    about = "Plain-text, version-controlled task management",
    long_about = "\
Plain-text, version-controlled task management.

Reads all *.agile.md files found anywhere under the current directory.
Files are prioritised by their path relative to the project root, so
tasks/50_current/001.agile.md outranks tasks/60_backlog/001.agile.md.

Run without a subcommand to open the next task in $VISUAL / $EDITOR.",
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
enum ListWhat {
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
enum TaskAction {
    /// Show the next highest-priority incomplete task
    ///
    /// Returns the first incomplete ([ ]) top-level task across all task
    /// files in priority order, including its full subtask tree. Skips
    /// done ([x]) and cancelled ([-]) tasks. Prints nothing if every
    /// task is complete or cancelled.
    Next,
}

fn open_editor(path: &std::path::Path, line: usize) {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| {
            eprintln!("agile: neither $VISUAL nor $EDITOR is set");
            std::process::exit(1);
        });
    let args = editor_open_args(&editor, path, line);
    let status = std::process::Command::new(&editor).args(&args).status();
    if let Err(e) = status {
        eprintln!("agile: failed to launch editor '{editor}': {e}");
        std::process::exit(1);
    }
}

fn main() {
    let root = Path::new(".");
    match Cli::parse().command {
        None => match mdagile::find_file_with_next_task(root) {
            Some(path) => {
                let line = mdagile::find_next_task_line(&path).unwrap_or(1);
                open_editor(&path, line);
            }
            None => eprintln!("agile: no active tasks found"),
        },
        Some(Command::List { what: None, next, last, all })
        | Some(Command::List { what: Some(ListWhat::Tasks { next, last, all }), .. }) => {
            let items = mdagile::parse_files(&mdagile::find_task_files(root));
            let blocks = if all {
                mdagile::list_task_blocks(&items)
            } else {
                mdagile::active_task_blocks(&items)
            };
            let result: String = apply_limit(blocks, next, last).into_iter().collect();
            print!("{result}");
        }
        Some(Command::List { what: Some(ListWhat::Files { next, last }), .. }) => {
            let paths = mdagile::find_task_files(root);
            let limited = apply_limit(paths, next, last);
            print!("{}", mdagile::format_file_list(&limited));
        }
        Some(Command::Task { action: TaskAction::Next }) => {
            let items = mdagile::parse_files(&mdagile::find_task_files(root));
            print!("{}", mdagile::next_task(&items));
        }
        Some(Command::Check) => {
            let items = mdagile::parse_files(&mdagile::find_task_files(root));
            let issues = mdagile::checker::run(&items);
            for issue in &issues {
                print!("{}", mdagile::format_issue(issue));
            }
            if !issues.is_empty() {
                std::process::exit(1);
            }
        }
    }
}

fn editor_open_args(editor: &str, path: &std::path::Path, line: usize) -> Vec<std::ffi::OsString> {
    use std::ffi::OsString;
    let bin_name = std::path::Path::new(editor)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    match bin_name.as_ref() {
        "vim" | "vi" | "nvim" | "nano" | "emacs" => {
            vec![OsString::from(format!("+{line}")), path.into()]
        }
        "code" => vec![
            OsString::from("--goto"),
            OsString::from(format!("{}:{line}", path.display())),
        ],
        _ => vec![path.into()],
    }
}

fn apply_limit<T>(items: Vec<T>, next: Option<usize>, last: Option<usize>) -> Vec<T> {
    match (next, last) {
        (Some(n), _) => items.into_iter().take(n).collect(),
        (_, Some(n)) => { let skip = items.len().saturating_sub(n); items.into_iter().skip(skip).collect() }
        (None, None) => items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::Path;

    fn s(v: &str) -> OsString { OsString::from(v) }

    #[test]
    fn vim_uses_plus_line() {
        assert_eq!(editor_open_args("vim", Path::new("f.agile.md"), 5), vec![s("+5"), s("f.agile.md")]);
    }

    #[test]
    fn vi_uses_plus_line() {
        assert_eq!(editor_open_args("vi", Path::new("f.agile.md"), 1), vec![s("+1"), s("f.agile.md")]);
    }

    #[test]
    fn nvim_uses_plus_line() {
        assert_eq!(editor_open_args("nvim", Path::new("f.agile.md"), 3), vec![s("+3"), s("f.agile.md")]);
    }

    #[test]
    fn nano_uses_plus_line() {
        assert_eq!(editor_open_args("nano", Path::new("f.agile.md"), 7), vec![s("+7"), s("f.agile.md")]);
    }

    #[test]
    fn emacs_uses_plus_line() {
        assert_eq!(editor_open_args("emacs", Path::new("f.agile.md"), 2), vec![s("+2"), s("f.agile.md")]);
    }

    #[test]
    fn code_uses_goto_flag() {
        assert_eq!(editor_open_args("code", Path::new("f.agile.md"), 4), vec![s("--goto"), s("f.agile.md:4")]);
    }

    #[test]
    fn full_path_uses_basename_for_matching() {
        assert_eq!(editor_open_args("/usr/bin/nvim", Path::new("f.agile.md"), 9), vec![s("+9"), s("f.agile.md")]);
    }

    #[test]
    fn unknown_editor_omits_line_number() {
        assert_eq!(editor_open_args("gedit", Path::new("f.agile.md"), 6), vec![s("f.agile.md")]);
    }

    #[test]
    fn tasks_is_alias_for_task_subcommand() {
        let cli = Cli::try_parse_from(["agile", "tasks", "next"])
            .expect("`agile tasks next` should parse as the `task next` subcommand");
        assert!(matches!(
            cli.command,
            Some(Command::Task { action: TaskAction::Next })
        ));
    }
}
