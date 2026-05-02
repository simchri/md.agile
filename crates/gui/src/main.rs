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

#[server]
async fn validate_working_dir() -> Result<(), ServerFnError> {
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

    match working_dir_res {
        Ok(dir) => check_for_mdagile_toml(&dir),
        Err(e) => {
            error!("could not determine working directory: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn check_for_mdagile_toml(dir: &Path) {
    if dir.join("mdagile.toml").is_file() {
        info!("found project root at {}", dir.display());
    } else {
        error!("could not find project root (no mdagile.toml found in {})", dir.display());
        std::process::exit(1);
    }
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
    
    rsx! {
        div { class: "layout",
            div { class: "separator1" }
            div { class: "separator2" }

            div { class: "task-card", style: "top: 30px; left: 30px;",
                "foo"
            }
        }
    }
}
