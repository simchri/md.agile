use dioxus::prelude::*;
use log::info;
use log::error;
use std::path::PathBuf;
use std::sync::Mutex;

static WORKING_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Returns the title of the next highest-priority incomplete task.
///
/// Runs on the server (the local dev process), reads `*.agile.md` from the
/// project root, and yields the title of the first `[ ]` top-level task.
#[server]
pub async fn get_next_task() -> Result<Option<String>, ServerFnError> {
    use mdagile::cli::common::{find_task_files, parse_files};
    use mdagile::cli::subcommands::task::next_task_title;

    let root = get_or_init_working_dir()?;
    let items = parse_files(&find_task_files(&root));
    Ok(next_task_title(&items))
}

fn get_or_init_working_dir() -> Result<PathBuf, ServerFnError> {
    let mut cached = WORKING_DIR.lock().unwrap();

    if let Some(dir) = cached.as_ref() {
        return Ok(dir.clone());
    }

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
                return Err(ServerFnError::new("mdagile.toml not found"));
            }
        }
        Err(e) => {
            error!("could not determine working directory: {e}");
            return Err(ServerFnError::new("failed to determine working directory"));
        }
    };

    *cached = Some(dir.clone());
    Ok(dir)
}

