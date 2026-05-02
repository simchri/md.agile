use serde;
use dioxus::prelude::*;
use log::info;
use log::error;
use notify::{Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use once_cell::sync::Lazy;
use dioxus_fullstack::ServerEvents;
use tokio::sync::broadcast;

use dioxus::prelude::*;


fn main() {
    init_logger();
    info!("mdagile-gui main");

    dioxus::launch(app);
}

fn init_logger() {
    #[cfg(not(feature = "web"))]
    env_logger::init();

    #[cfg(feature = "web")]
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");
}

/// Returns the title of the next highest-priority incomplete task.
///
/// Runs on the server (the local dev process), reads `*.agile.md` from the
/// project root, and yields the title of the first `[ ]` top-level task.
#[server]
async fn get_next_task() -> Result<Option<String>, ServerFnError> {
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

fn format_task_title(result: &Option<Result<Option<String>, ServerFnError>>) -> String {
    match result {
        Some(Ok(Some(t))) => t.clone(),
        Some(Ok(None))    => "All tasks done".to_string(),
        Some(Err(e))      => format!("Error: {e}"),
        None              => "Loading…".to_string(),
    }
}

fn app() -> Element {
    
    let mut next = use_resource(|| async { get_next_task().await });

    let title = match &*next.read_unchecked() {
        Some(Ok(Some(t))) => t.clone(),
        Some(Ok(None))    => "All tasks done".to_string(),
        Some(Err(e))      => format!("Error: {e}"),
        None              => "Loading…".to_string(),
    };

    use_effect({
        // Clock, frequency 1s.
        // Poll updates from the server side (e.g. update task list)
        move || {
            dioxus::prelude::spawn(async move {
                log::info!("use_effect: clock START");
                use wasmtimer::tokio::sleep;

                loop {
                    sleep(std::time::Duration::from_millis(1000)).await;

                    // updates the "next" resource, by re-evaluating the registered function, therefore calling the backend, to get the current latest task.
                    next.restart(); 
                }
            });
        }
    });

    rsx! {
        div { class: "layout",
            div { class: "separator1" }
            div { class: "separator2" }

            div { class: "task-card", style: "top: 30px; left: 30px;",
                "{title}"
            }
        }
    }
}
