use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(name = "agile", about = "Mdagile task management")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List tasks or files
    List {
        #[command(subcommand)]
        what: Option<ListWhat>,
    },
    /// Task subcommands
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
}

#[derive(Subcommand)]
enum ListWhat {
    /// Show recognized task files in priority order
    Files,
}

#[derive(Subcommand)]
enum TaskAction {
    /// Show the next highest-priority incomplete task
    Next,
}

fn main() {
    let root = Path::new(".");
    match Cli::parse().command.unwrap_or(Command::List { what: None }) {
        Command::List { what: None } => {
            print!("{}", mdagile::list_tasks(&mdagile::read_task_files(root)));
        }
        Command::List { what: Some(ListWhat::Files) } => {
            for path in mdagile::find_task_files(root) {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                println!("{name}  {}", path.display());
            }
        }
        Command::Task { action: TaskAction::Next } => {
            print!("{}", mdagile::next_task(&mdagile::read_task_files(root)));
        }
    }
}
