use serde;
use dioxus::prelude::*;
use log::info;
use log::error;
use std::path::{PathBuf};

/// Returns the title of the next highest-priority incomplete task.
///
/// Runs on the server (the local dev process), reads `*.agile.md` from the
/// project root, and yields the title of the first `[ ]` top-level task.
#[server]
pub async fn get_next_task() -> Result<Option<String>, ServerFnError> {
    use mdagile::cli::common::{find_task_files, parse_files};
    use mdagile::cli::subcommands::task::next_task_title;

    // dx's CWD is the gui crate dir, not the project root. Walk up to find
    // the directory containing `mdagile.toml` (the project marker).
    let root_dir_opt = get_validated_working_dir().await?;

    let root = match root_dir_opt {
        Some(dir) => dir,
        None => return Err(ServerFnError::new("working directory not found")),
    };


    let items = parse_files(&find_task_files(&root));
    Ok(next_task_title(&items))
}

#[server]
async fn get_validated_working_dir() -> Result<Option<PathBuf>, ServerFnError> {
    info!("checking for mdagile.toml file..");
    let working_dir_res = std::env::var("MDAGILE_WORKDIR")
        .map(PathBuf::from)
        .or_else(|_| {
            let mut args = std::env::args().skip(1);
            if let Some(arg) = args.next() {
                Ok(PathBuf::from(arg))
            } else {
                std::env::current_dir()
            }
        });

    let dir = match working_dir_res {
        Ok(dir) => {
            if dir.join("mdagile.toml").is_file() {
                info!("found project root at {}", dir.display());
                dir
            } else {
                error!("could not find project root (no mdagile.toml found in {})", dir.display());
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("could not determine working directory: {e}");
            std::process::exit(1);
        }
    };

    Ok(Some(dir))
}
