use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use {
    log::{error, info},
    std::path::{Path, PathBuf},
    std::sync::Mutex,
};

#[cfg(feature = "server")]
static WORKING_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Cached result of [`is_kiosk_mode`]. `None` means "not yet resolved".
#[cfg(feature = "server")]
static KIOSK_MODE: Mutex<Option<bool>> = Mutex::new(None);

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
///
/// `path` (relative to the project root, forward-slash separated) and `line`
/// (1-based) together identify exactly where this node lives in its source
/// file — the same [`mdagile::parser::Location`] the parser already tracks
/// internally. This is the handle [`mark_task_done`] uses to locate and
/// mutate the right line without any separate addressing scheme.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TaskView {
    pub status: TaskStatus,
    pub title: String,
    pub markers: Vec<String>,
    pub body: Vec<String>,
    pub children: Vec<TaskView>,
    /// The explicit sibling order number (the `1` in `- [ ] 1. do this`), if
    /// this node is an ordered subtask. Always `None` for top-level tasks —
    /// only subtasks can be ordered — and for subtasks without an order
    /// prefix at all. Lets the UI both display the order and (via the
    /// server-side completion check) refuse to mark a lower-ordered
    /// sibling's task done out of turn.
    pub order: Option<u32>,
    pub rank: usize,
    pub path: String,
    pub line: usize,
}

#[cfg(feature = "server")]
const BACKLOG_LIMIT: usize = 10;
#[cfg(feature = "server")]
const IN_PROGRESS_LIMIT: usize = 40;
#[cfg(feature = "server")]
const DONE_LIMIT: usize = 10;

/// Retrieves and categorizes tasks from the working directory.
///
/// This server function scans the working directory for task files, parses them,
/// and organizes the resulting tasks into three categories:
/// - In Progress: Tasks that have started, up to a configured limit.
/// - Backlog: Tasks that are yet to be started, up to a configured limit.
/// - Done: Recently completed tasks, limited to the most recent ones.
///
/// Tasks are ranked in the order they are found. The function returns a single
/// vector containing all tasks, with in-progress tasks first, followed by backlog,
/// and then done tasks.
///
/// # Returns
/// - `Ok(Vec<TaskView>)`: A vector of task views in the specified order.
/// - `Err(ServerFnError)`: If there is an error initializing the working directory or reading files.
///
/// # Errors
/// Returns an error if the working directory cannot be initialized or if task files cannot be parsed.
///
/// # Example
/// ```ignore
/// let tasks = get_tasks().await?;
/// ```
#[server]
pub async fn get_tasks() -> Result<Vec<TaskView>, ServerFnError> {
    use mdagile::cli::common::{find_task_files, parse_files};
    use mdagile::parser::{FileItem, Status};

    let root = get_or_init_working_dir()?;
    log::info!("scanning for tasks in {}", root.display());
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
                    let view = task_to_view(task, task_rank, &root);
                    if crate::card_positioning::has_started(&view) {
                        if in_progress.len() < IN_PROGRESS_LIMIT {
                            in_progress.push(view);
                        }
                    } else if backlog.len() < BACKLOG_LIMIT {
                        backlog.push(view);
                    }
                }
                Status::Done => dones.push(task_to_view(task, task_rank, &root)),
                Status::Cancelled => {}
            }
        }
    }

    let skip = dones.len().saturating_sub(DONE_LIMIT);
    let done: Vec<TaskView> = dones.into_iter().skip(skip).collect();

    log::info!(
        "tasks found : in_progress={}, backlog={}, done={}",
        in_progress.len(),
        backlog.len(),
        done.len()
    );

    // concatenate the three categories into one vector:
    let tasks = in_progress
        .into_iter()
        .chain(backlog.into_iter())
        .chain(done.into_iter())
        .collect();

    Ok(tasks)
}

#[cfg(feature = "server")]
fn task_to_view(task: &mdagile::parser::Task, rank: usize, root: &Path) -> TaskView {
    TaskView {
        status: status_to_view(&task.status),
        title: task.title.clone(),
        markers: task.markers.iter().map(format_marker).collect(),
        body: task.body.clone(),
        children: task
            .children
            .iter()
            .map(|s| subtask_to_view(s, root))
            .collect(),
        order: None,
        rank,
        path: relative_path(&task.location.path, root),
        line: task.location.line,
    }
}

#[cfg(feature = "server")]
fn subtask_to_view(sub: &mdagile::parser::Subtask, root: &Path) -> TaskView {
    TaskView {
        status: status_to_view(&sub.status),
        title: sub.title.clone(),
        markers: sub.markers.iter().map(format_marker).collect(),
        body: sub.body.clone(),
        children: sub
            .children
            .iter()
            .map(|s| subtask_to_view(s, root))
            .collect(),
        order: match sub.order {
            mdagile::parser::Order::Ordered(n) => Some(n),
            mdagile::parser::Order::None => None,
        },
        rank: 0,
        path: relative_path(&sub.location.path, root),
        line: sub.location.line,
    }
}

/// Renders `path` relative to `root` as a forward-slash-separated string
/// suitable for round-tripping to [`mark_task_done`] — never an absolute
/// path, so the client never sees (or needs to send back) filesystem layout
/// outside the project.
#[cfg(feature = "server")]
fn relative_path(path: &Path, root: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
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
    use mdagile::parser::Marker;
    match marker {
        Marker::Property(p) => format!("#{}", p.name),
        Marker::Assignment(a) => format!("@{}", a.name),
        Marker::Special(s) => format!("#{}", s.as_str()),
    }
}

#[cfg(feature = "server")]
fn get_or_init_working_dir() -> Result<PathBuf, ServerFnError> {
    fn is_file_or_symlink(path: &PathBuf) -> bool {
        use std::fs;
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.is_file() {
                    true
                } else if metadata.file_type().is_symlink() {
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

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
            let toml_path = dir.join("mdagile.toml");
            if is_file_or_symlink(&toml_path) {
                info!("found project root at {}", dir.display());
                dir
            } else {
                error!(
                    "could not find project root (no mdagile.toml found in {})",
                    dir.display()
                );
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

/// Resolves and caches whether the GUI is running in kiosk mode (write
/// actions disabled), analogous to [`get_or_init_working_dir`]'s handling of
/// `MDAGILE_WORKDIR`: read once from the `MDAGILE_KIOSK` env var (any
/// non-empty value other than `"0"`/`"false"` enables it), then cached for
/// the lifetime of the server process.
#[cfg(feature = "server")]
fn is_kiosk_mode() -> bool {
    let mut cached = KIOSK_MODE.lock().unwrap();
    if let Some(kiosk) = *cached {
        return kiosk;
    }

    let kiosk = std::env::var("MDAGILE_KIOSK")
        .map(|v| !v.is_empty() && v != "0" && v.to_lowercase() != "false")
        .unwrap_or(false);
    *cached = Some(kiosk);
    kiosk
}

/// Reports whether the server is running in kiosk mode, so the frontend can
/// hide write-capable UI (e.g. the "mark done" button). This is purely a UX
/// convenience — [`mark_task_done`] independently re-checks kiosk mode
/// server-side before making any change, since the client can't be trusted
/// to honor a hidden button.
#[server]
pub async fn get_kiosk_mode() -> Result<bool, ServerFnError> {
    Ok(is_kiosk_mode())
}

/// Marks the task or subtask identified by `path` (relative to the project
/// root, as returned in a [`TaskView`]) and `line` done.
///
/// Reuses exactly the same core logic (and completion rules) as `agile task
/// done` — see [`mdagile::cli::subcommands::task::mark_node_done`] — so a
/// task that the CLI would refuse to complete (e.g. incomplete required
/// children) is refused here too, with the same reasons.
#[server]
pub async fn mark_task_done(path: String, line: usize) -> Result<(), ServerFnError> {
    use mdagile::cli::common::{find_task_files, parse_file};
    use mdagile::cli::subcommands::task::{mark_node_done, MarkDoneError};

    if is_kiosk_mode() {
        return Err(ServerFnError::new("kiosk mode: write actions are disabled"));
    }

    let root = get_or_init_working_dir()?;
    let file = root.join(&path);

    // Only accept paths `find_task_files` itself would discover — this is
    // also what prevents a stale or malicious client from pointing this
    // write action at an arbitrary path outside the project.
    if !find_task_files(&root).iter().any(|f| *f == file) {
        return Err(ServerFnError::new("not a recognized task file"));
    }

    let config = mdagile::config::Config::load(&root)
        .map_err(|e| ServerFnError::new(format!("could not load config: {e}")))?;
    let items = parse_file(&file);

    match mark_node_done(&file, &items, line, &config) {
        Ok(_title) => Ok(()),
        Err(MarkDoneError::NotFound) => Err(ServerFnError::new(
            "task changed on disk — please refresh and try again",
        )),
        Err(MarkDoneError::NotTodo(title)) => {
            Err(ServerFnError::new(format!("\"{title}\" is already done")))
        }
        Err(MarkDoneError::RuleViolations(issues)) => {
            let messages: Vec<String> = issues.iter().map(|i| i.message.clone()).collect();
            Err(ServerFnError::new(messages.join("; ")))
        }
        Err(MarkDoneError::Io(e)) => Err(ServerFnError::new(e)),
    }
}

/// Reverts the task or subtask identified by `path` (relative to the
/// project root, as returned in a [`TaskView`]) and `line` back to todo.
///
/// Reuses exactly the same core logic as `agile task undone` — see
/// [`mdagile::cli::subcommands::task::mark_node_undone`] — so, like the
/// CLI, there are no completion rules to satisfy in reverse: a done task
/// can always be un-done regardless of its parent's or children's state.
#[server]
pub async fn mark_task_undone(path: String, line: usize) -> Result<(), ServerFnError> {
    use mdagile::cli::common::{find_task_files, parse_file};
    use mdagile::cli::subcommands::task::{mark_node_undone, MarkUndoneError};

    if is_kiosk_mode() {
        return Err(ServerFnError::new("kiosk mode: write actions are disabled"));
    }

    let root = get_or_init_working_dir()?;
    let file = root.join(&path);

    // Only accept paths `find_task_files` itself would discover — this is
    // also what prevents a stale or malicious client from pointing this
    // write action at an arbitrary path outside the project.
    if !find_task_files(&root).iter().any(|f| *f == file) {
        return Err(ServerFnError::new("not a recognized task file"));
    }

    let items = parse_file(&file);

    match mark_node_undone(&file, &items, line) {
        Ok(_title) => Ok(()),
        Err(MarkUndoneError::NotFound) => Err(ServerFnError::new(
            "task changed on disk — please refresh and try again",
        )),
        Err(MarkUndoneError::NotDone(title)) => {
            Err(ServerFnError::new(format!("\"{title}\" is not done")))
        }
        Err(MarkUndoneError::Io(e)) => Err(ServerFnError::new(e)),
    }
}
