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
///
/// `rank` is the priority of a top-level task: 0 = first task encountered when
/// parsing all `.agile.md` files in the project (file path order, then
/// in-file order). It defines the visual ordering on the canvas regardless of
/// which slot a task happens to occupy. Subtasks carry `rank == 0` since they
/// are rendered inline within their parent and never positioned independently.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TaskView {
    pub status: TaskStatus,
    pub title: String,
    pub markers: Vec<String>,
    pub body: Vec<String>,
    pub children: Vec<TaskView>,
    pub rank: usize,
}

/// Tasks bundle delivered to the GUI on every poll. The frontend renders a
/// post-it for each entry; the bucket determines where it sits on the canvas:
/// `in_progress` floats in the middle along the diagonal, `backlog` lives in
/// the top row, `done` in the bottom row.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TaskList {
    pub in_progress: Vec<TaskView>,
    pub backlog: Vec<TaskView>,
    pub done: Vec<TaskView>,
}

#[cfg(feature = "server")]
const BACKLOG_LIMIT: usize = 10;
#[cfg(feature = "server")]
const IN_PROGRESS_LIMIT: usize = 10;
#[cfg(feature = "server")]
const DONE_LIMIT: usize = 10;

/// Returns Todo tasks split by progress (in_progress vs. backlog) and the
/// last [`DONE_LIMIT`] completed tasks.
///
/// A Todo task counts as `in_progress` once at least one of its direct
/// subtasks is Done or Cancelled — i.e. work has begun. Otherwise it stays
/// in `backlog`.
#[server]
pub async fn get_tasks() -> Result<TaskList, ServerFnError> {
    use mdagile::cli::common::{find_task_files, parse_files};
    use mdagile::parser::{FileItem, Status};

    let root = get_or_init_working_dir()?;
    let items = parse_files(&find_task_files(&root));

    let mut in_progress = Vec::new();
    let mut backlog = Vec::new();
    let mut dones = Vec::new();

    let mut rank: usize = 0;
    for item in items.iter() {
        if let FileItem::Task(task) = item {
            let task_rank = rank;
            rank += 1;
            match task.status {
                Status::Todo => {
                    let view = task_to_view(task, task_rank);
                    if has_started(&view) {
                        if in_progress.len() < IN_PROGRESS_LIMIT {
                            in_progress.push(view);
                        }
                    } else if backlog.len() < BACKLOG_LIMIT {
                        backlog.push(view);
                    }
                }
                Status::Done => dones.push(task_to_view(task, task_rank)),
                Status::Cancelled => {}
            }
        }
    }

    let skip = dones.len().saturating_sub(DONE_LIMIT);
    let done: Vec<TaskView> = dones.into_iter().skip(skip).collect();

    Ok(TaskList { in_progress, backlog, done })
}

/// True when the task has at least one direct subtask marked Done or
/// Cancelled — i.e. work has begun on it. Used both for server-side
/// categorization and frontend size-class selection.
pub fn has_started(task: &TaskView) -> bool {
    task.children
        .iter()
        .any(|c| matches!(c.status, TaskStatus::Done | TaskStatus::Cancelled))
}

#[cfg(feature = "server")]
fn task_to_view(task: &mdagile::parser::Task, rank: usize) -> TaskView {
    TaskView {
        status: status_to_view(&task.status),
        title: task.title.clone(),
        markers: task.markers.iter().map(format_marker).collect(),
        body: task.body.clone(),
        children: task.children.iter().map(subtask_to_view).collect(),
        rank,
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
        rank: 0,
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
