use dioxus::prelude::*;
use log::info;

mod server;

use server::{TaskList, TaskStatus, TaskView};

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

fn app() -> Element {
    let mut tasks = use_resource(|| async { server::get_tasks().await });
    let mut modal_open = use_signal(|| false);

    use_effect({
        // Clock, frequency 1s.
        // Poll updates from the server side (e.g. update task list)
        move || {
            dioxus::prelude::spawn(async move {
                log::info!("use_effect: clock START");
                use wasmtimer::tokio::sleep;

                loop {
                    sleep(std::time::Duration::from_millis(1000)).await;
                    tasks.restart();
                }
            });
        }
    });

    let (current_task, backlog): (Option<TaskView>, Vec<TaskView>) = match &*tasks.read_unchecked() {
        Some(Ok(TaskList { current, backlog })) => (current.clone(), backlog.clone()),
        _ => (None, Vec::new()),
    };

    let card = match &*tasks.read_unchecked() {
        Some(Ok(TaskList { current: Some(t), .. })) => rsx! {
            TaskCard {
                task: t.clone(),
                on_click: move |_| modal_open.set(true),
            }
        },
        Some(Ok(TaskList { current: None, .. })) => rsx! { div { class: "task-card", style: "{diagonal_style(1.0)}", "All tasks done" } },
        Some(Err(e))                              => rsx! { div { class: "task-card", style: "{diagonal_style(0.0)}", "Error: {e}" } },
        None                                      => rsx! { div { class: "task-card", style: "{diagonal_style(0.0)}", "Loading…" } },
    };

    rsx! {
        div { class: "layout",
            for (i, task) in backlog.iter().enumerate() {
                BacklogCard { task: task.clone(), index: i }
            }
            div { class: "separator1" }
            div { class: "separator2" }
            {card}

            if modal_open() {
                if let Some(task) = current_task {
                    TaskModal {
                        task: task,
                        on_close: move |_| modal_open.set(false),
                    }
                }
            }
        }
    }
}

#[component]
fn TaskCard(task: TaskView, on_click: EventHandler<MouseEvent>) -> Element {
    let progress = task_progress(&task);
    rsx! {
        div { class: "task-card", style: "{diagonal_style(progress)}",
            onclick: move |evt| on_click.call(evt),
            div { class: "task-card-header",
                span { class: status_class(&task.status), {status_box(&task.status)} }
                span { class: "task-card-title", "{task.title}" }
            }

            if !task.markers.is_empty() {
                div { class: "task-card-markers",
                    for marker in &task.markers {
                        span { class: "marker", "{marker}" }
                    }
                }
            }

            if !task.body.is_empty() {
                div { class: "task-card-body",
                    for line in &task.body {
                        div { "{line}" }
                    }
                }
            }

            if !task.children.is_empty() {
                ul { class: "task-card-children",
                    for child in &task.children {
                        SubtaskItem { task: child.clone(), depth: 1, show_body: false }
                    }
                }
            }
        }
    }
}

/// Horizontal step (in px) between two adjacent backlog post-its. The card
/// itself is 110px wide; the extra 10px is the visual gap between cards.
const BACKLOG_OFFSET_PX: usize = 120;
const BACKLOG_LEFT_PX: usize = 12;

#[component]
fn BacklogCard(task: TaskView, index: usize) -> Element {
    let style = format!("left: {}px;", BACKLOG_LEFT_PX + index * BACKLOG_OFFSET_PX);
    rsx! {
        div { class: "backlog-card", style: "{style}",
            div { class: "backlog-card-status {status_class(&task.status)}",
                {status_box(&task.status)}
            }
            div { class: "backlog-card-title", "{task.title}" }
            if !task.markers.is_empty() {
                div { class: "backlog-card-markers",
                    for marker in &task.markers {
                        span { class: "marker", "{marker}" }
                    }
                }
            }
        }
    }
}

#[component]
fn SubtaskItem(task: TaskView, depth: usize, show_body: bool) -> Element {
    let style = format!("padding-left: {}px;", (depth - 1) * 8);
    rsx! {
        li { class: "subtask {status_class(&task.status)}", style: "{style}",
            span { class: "subtask-status", {status_box(&task.status)} }
            span { class: "subtask-title", "{task.title}" }
            if !task.markers.is_empty() {
                span { class: "subtask-markers",
                    for marker in &task.markers {
                        span { class: "marker", "{marker}" }
                    }
                }
            }
            if show_body && !task.body.is_empty() {
                div { class: "subtask-body",
                    for line in &task.body {
                        div { "{line}" }
                    }
                }
            }
            if !task.children.is_empty() {
                ul { class: "subtask-children",
                    for child in &task.children {
                        SubtaskItem { task: child.clone(), depth: depth + 1, show_body: show_body }
                    }
                }
            }
        }
    }
}

#[component]
fn TaskModal(task: TaskView, on_close: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div { class: "modal-backdrop",
            onclick: move |evt| on_close.call(evt),

            div { class: "modal-content",
                onclick: move |evt: MouseEvent| evt.stop_propagation(),

                button { class: "modal-close",
                    onclick: move |evt| on_close.call(evt),
                    "×"
                }

                div { class: "modal-task-header",
                    span { class: "modal-status {status_class(&task.status)}",
                        {status_box(&task.status)}
                    }
                    h1 { class: "modal-task-title", "{task.title}" }
                }

                if !task.markers.is_empty() {
                    div { class: "modal-markers",
                        for marker in &task.markers {
                            span { class: "marker", "{marker}" }
                        }
                    }
                }

                if !task.body.is_empty() {
                    div { class: "modal-body",
                        for line in &task.body {
                            div { "{line}" }
                        }
                    }
                }

                if !task.children.is_empty() {
                    ul { class: "modal-children",
                        for child in &task.children {
                            SubtaskItem { task: child.clone(), depth: 1, show_body: true }
                        }
                    }
                }
            }
        }
    }
}

/// Returns the share of subtasks that are complete (Done or Cancelled), counted
/// recursively across all nesting levels. A task with no subtasks reports 0.0,
/// and a task whose own status is already Done reports 1.0.
fn task_progress(task: &TaskView) -> f64 {
    if matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled) {
        return 1.0;
    }
    let (done, total) = count_subtasks(task);
    if total == 0 {
        0.0
    } else {
        done as f64 / total as f64
    }
}

fn count_subtasks(task: &TaskView) -> (usize, usize) {
    let mut done = 0;
    let mut total = 0;
    for child in &task.children {
        total += 1;
        if matches!(child.status, TaskStatus::Done | TaskStatus::Cancelled) {
            done += 1;
        }
        let (cd, ct) = count_subtasks(child);
        done += cd;
        total += ct;
    }
    (done, total)
}

/// Builds a CSS positioning rule that places the post-it along the top-left to
/// bottom-right diagonal at `progress` (0.0 = top-left, 1.0 = bottom-right).
/// 280px = 220px card + 60px combined margin from the viewport edges.
fn diagonal_style(progress: f64) -> String {
    let p = progress.clamp(0.0, 1.0);
    format!(
        "top: calc({p:.3} * (100vh - 280px) + 30px); left: calc({p:.3} * (100vw - 280px) + 30px);"
    )
}

fn status_box(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo      => "[ ]",
        TaskStatus::Done      => "[x]",
        TaskStatus::Cancelled => "[-]",
    }
}

fn status_class(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo      => "status-todo",
        TaskStatus::Done      => "status-done",
        TaskStatus::Cancelled => "status-cancelled",
    }
}
