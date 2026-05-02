use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use {
    log::{error, info},
    std::path::PathBuf,
    std::sync::Mutex,
};

#[cfg(feature = "server")]
static WORKING_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Todo,
    Done,
    Cancelled,
}

/// A frontend-friendly view of a task or subtask.
///
/// `Task` and `Subtask` from the parser are merged into one type here because
/// the UI renders them the same way — only the styling differs by depth.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TaskView {
    pub status: TaskStatus,
    pub title: String,
    pub markers: Vec<String>,
    pub body: Vec<String>,
    pub children: Vec<TaskView>,
}

/// Returns the next highest-priority incomplete task with all its subtree info.
///
/// Runs on the server (the local dev process), reads `*.agile.md` from the
/// project root, and yields the first `[ ]` top-level task as a [`TaskView`].
#[server]
pub async fn get_next_task() -> Result<Option<TaskView>, ServerFnError> {
    use mdagile::cli::common::{find_task_files, parse_files};
    use mdagile::parser::{FileItem, Status};

    let root = get_or_init_working_dir()?;
    let items = parse_files(&find_task_files(&root));

    let next = items.iter().find_map(|item| match item {
        FileItem::Task(task) if task.status == Status::Todo => Some(task_to_view(task)),
        _ => None,
    });

    Ok(next)
}

#[cfg(feature = "server")]
fn task_to_view(task: &mdagile::parser::Task) -> TaskView {
    TaskView {
        status: status_to_view(&task.status),
        title: task.title.clone(),
        markers: task.markers.iter().map(format_marker).collect(),
        body: task.body.clone(),
        children: task.children.iter().map(subtask_to_view).collect(),
    }
}

#[cfg(feature = "server")]
fn subtask_to_view(sub: &mdagile::parser::Subtask) -> TaskView {
    TaskView {
        status: status_to_view(&sub.status),
        title: sub.title.clone(),
        markers: sub.markers.iter().map(format_marker).collect(),
        body: sub.body.clone(),
        children: sub.children.iter().map(subtask_to_view).collect(),
    }
}

#[cfg(feature = "server")]
fn status_to_view(status: &mdagile::parser::Status) -> TaskStatus {
    match status {
        mdagile::parser::Status::Todo => TaskStatus::Todo,
        mdagile::parser::Status::Done => TaskStatus::Done,
        mdagile::parser::Status::Cancelled => TaskStatus::Cancelled,
    }
}

#[cfg(feature = "server")]
fn format_marker(marker: &mdagile::parser::Marker) -> String {
    use mdagile::parser::{Marker, SpecialMarker};
    match marker {
        Marker::Property(p) => format!("#{}", p.name),
        Marker::Assignment(name) => format!("@{}", name),
        Marker::Special(SpecialMarker::Opt) => "#OPT".to_string(),
        Marker::Special(SpecialMarker::Milestone) => "#MILESTONE".to_string(),
        Marker::Special(SpecialMarker::MdAgile) => "#MDAGILE".to_string(),
    }
}

#[cfg(feature = "server")]
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
