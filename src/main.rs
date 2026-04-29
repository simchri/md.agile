use clap::{Parser, Subcommand};
use ignore::WalkBuilder;
use std::path::Path;

#[derive(Parser)]
#[command(name = "agile", about = "Mdagile task management")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show all tasks
    List,
    /// Task subcommands
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
}

#[derive(Subcommand)]
enum TaskAction {
    /// Show the next highest-priority incomplete task
    Next,
}

fn main() {
    let cli = Cli::parse();
    let input = collect_task_files(".");
    let output = match cli.command.unwrap_or(Command::List) {
        Command::List => mdagile::list_tasks(&input),
        Command::Task { action: TaskAction::Next } => mdagile::next_task(&input),
    };
    print!("{output}");
}

fn collect_task_files(root: impl AsRef<Path>) -> String {
    let mut paths: Vec<_> = WalkBuilder::new(root)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().map(|t| t.is_file()).unwrap_or(false)
                && e.file_name().to_string_lossy().ends_with(".agile.md")
        })
        .map(|e| e.into_path())
        .collect();

    paths.sort_by(|a, b| {
        a.file_name().cmp(&b.file_name())
    });

    paths
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect::<Vec<_>>()
        .join("\n")
}
