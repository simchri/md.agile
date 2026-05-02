use dioxus::prelude::*;
use log::info;

fn main() {
    #[cfg(not(feature = "web"))]
    env_logger::init();

    #[cfg(feature = "web")]
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");

    info!("main()");

    info!("Launch Dioxus App");
    dioxus::launch(App);
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
    let mut cur = std::env::current_dir().map_err(ServerFnError::new)?;
    let root = loop {
        if cur.join("mdagile.toml").is_file() { break cur; }
        match cur.parent() {
            Some(p) => cur = p.to_path_buf(),
            None => return Err(ServerFnError::new("project root (mdagile.toml) not found")),
        }
    };

    let items = parse_files(&find_task_files(&root));
    Ok(next_task_title(&items))
}

#[component]
fn App() -> Element {
    let next = use_resource(|| async { get_next_task().await });

    let title = match &*next.read_unchecked() {
        Some(Ok(Some(t))) => t.clone(),
        Some(Ok(None))    => "All tasks done".to_string(),
        Some(Err(e))      => format!("Error: {e}"),
        None              => "Loading…".to_string(),
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
