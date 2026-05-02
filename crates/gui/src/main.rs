use dioxus::prelude::*;
use log::info;
use log::error;
use notify::{Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use once_cell::sync::Lazy;

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

fn main() {
    init_logger();
    info!("mdagile-gui main");

    validate_working_dir();
    spawn_file_watcher();

    info!("Launching Dioxus App");
    dioxus::launch(App);
}

fn init_logger() {
    #[cfg(not(feature = "web"))]
    env_logger::init();

    #[cfg(feature = "web")]
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");
}

fn validate_working_dir() {
    info!("checking for mdagile.toml file..");
    let working_dir = &*WORKING_DIR;
    if working_dir.join("mdagile.toml").is_file() {
        info!("found project root at {}", working_dir.display());
    } else {
        error!("could not find project root (no mdagile.toml found)");
        std::process::exit(1);
    }
}

fn spawn_file_watcher() {
    let (tx, rx) = std::sync::mpsc::channel::<Result<notify::Event, notify::Error>>();

    std::thread::spawn(move || {
        if let Err(e) = watch_events(tx) {
            log::error!("watcher error: {:?}", e);
        } else {
            log::info!("watcher exited successfully");
        }
    });

    std::thread::spawn(move || {
        for event in rx {
            match event {
                Ok(ev) => log::info!("file event: {:?}", ev),
                Err(e) => log::error!("watch error: {:?}", e),
            }
        }
    });
}

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
///
/// Runs on the server (the local dev process), reads `*.agile.md` from the
/// project root, and yields the title of the first `[ ]` top-level task.
#[server]
async fn get_next_task() -> Result<Option<String>, ServerFnError> {
    use mdagile::cli::common::{find_task_files, parse_files};
    use mdagile::cli::subcommands::task::next_task_title;


    let items = parse_files(&find_task_files(&WORKING_DIR));
    Ok(next_task_title(&items))
}

#[component]
fn App() -> Element {
    let next = use_resource(|| async { get_next_task().await });
    let title = format_task_title(&*next.read_unchecked());

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

fn format_task_title(result: &Option<Result<Option<String>, ServerFnError>>) -> String {
    match result {
        Some(Ok(Some(t))) => t.clone(),
        Some(Ok(None))    => "All tasks done".to_string(),
        Some(Err(e))      => format!("Error: {e}"),
        None              => "Loading…".to_string(),
    }
}
