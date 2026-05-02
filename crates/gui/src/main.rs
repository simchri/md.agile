use dioxus::prelude::*;
use log::error;
use log::info;
use log::warn;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::{path::Path, sync::mpsc};

#[cfg(not(feature = "web"))]
use notify::{Event, RecursiveMode, Watcher};

#[cfg(not(feature = "web"))]
pub static WORKING_DIR: Lazy<PathBuf> = Lazy::new(|| {
    std::env::var("MDAGILE_WORKDIR")
        .map(PathBuf::from)
        .or_else(|_| {
            let mut args = std::env::args().skip(1);
            if let Some(arg) = args.next() {
                Ok(PathBuf::from(arg))
            } else {
                std::env::current_dir()
            }
        })
        .expect("failed to get working directory")
});

#[cfg(feature = "web")]
pub static WORKING_DIR: Lazy<PathBuf> = Lazy::new(|| std::path::PathBuf::new());

fn main() {
    #[cfg(not(feature = "web"))]
    env_logger::init();

    #[cfg(feature = "web")]
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");

    info!("mdagile-gui main");
    // Set Up Root Dir
    // ======
    // root dir is static for the lifetime of the application
    //
    info!("check for mdagile.toml file..");
    let working_dir = &*WORKING_DIR;
    if working_dir.join("mdagile.toml").is_file() {
        info!("found project root at {}", working_dir.display());
    } else {
        warn!(
            "could not find project root (no mdagile.toml found in current or parent directories)"
        );
    }

    // File Watching
    // ======
    // Create a channel for notifications
    #[cfg(not(feature = "web"))]
    let (tx, rx) = std::sync::mpsc::channel::<Result<notify::Event, notify::Error>>();

    // Spawn the watcher in a separate thread
    #[cfg(not(feature = "web"))]
    setup_file_watcher();

    // Spawn a thread to handle incoming notifications (example: print them)
    #[cfg(not(feature = "web"))]
    receive_file_change_events(rx);

    log::info!("Launch Dioxus App");
    dioxus::launch(App);
}

#[cfg(not(feature = "web"))]
fn receive_file_change_events(rx: mpsc::Receiver<Result<notify::Event, notify::Error>>) {
    std::thread::spawn(move || {
        for event in rx {
            match event {
                Ok(ev) => log::info!("Received event: {:?}", ev),
                Err(e) => log::error!("Watch error: {:?}", e),
            }
        }
    });
}

#[cfg(not(feature = "web"))]
fn setup_file_watcher(tx: mpsc::Sender<Result<notify::Event, notify::Error>>) {
    std::thread::spawn(move || {
        if let Err(e) = watch_events(tx) {
            log::error!("watcher error: {:?}", e);
        } else {
            log::info!("watcher exited successfully");
        }
    });
}

#[cfg(not(feature = "web"))]
fn watch_events(tx: std::sync::mpsc::Sender<Result<Event, notify::Error>>) -> notify::Result<()> {
    log::info!("set up event watcher");

    let mut watcher = notify::recommended_watcher(move |res| {
        tx.send(res).unwrap();
    })?;

    watcher.watch(Path::new("."), RecursiveMode::Recursive)?;

    // Keep the thread alive
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

/// Returns the title of the next highest-priority incomplete task.
#[cfg(not(feature = "web"))]
fn get_next_task() -> Option<String> {
    use mdagile::cli::common::{find_task_files, parse_files};
    use mdagile::cli::subcommands::task::next_task_title;

    let items = parse_files(&find_task_files(&WORKING_DIR));
    next_task_title(&items)
}

#[cfg(feature = "web")]
fn get_next_task() -> Option<String> {
    Some("dummy task".to_string())
}

#[component]
fn App() -> Element {
    let next = use_signal(|| get_next_task());

    let title = match next() {
        Some(t) => t.clone(),
        None => "Loading…".to_string(),
    };

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
