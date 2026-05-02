use dioxus::prelude::*;
use log::info;

mod server;

use server::{TaskStatus, TaskView};

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
    let mut next = use_resource(|| async { server::get_next_task().await });

    use_effect({
        // Clock, frequency 1s.
        // Poll updates from the server side (e.g. update task list)
        move || {
            dioxus::prelude::spawn(async move {
                log::info!("use_effect: clock START");
                use wasmtimer::tokio::sleep;

                loop {
                    sleep(std::time::Duration::from_millis(1000)).await;
                    next.restart();
                }
            });
        }
    });

    let card = match &*next.read_unchecked() {
        Some(Ok(Some(t))) => rsx! { TaskCard { task: t.clone() } },
        Some(Ok(None))    => rsx! { div { class: "task-card", style: "top: 30px; left: 30px;", "All tasks done" } },
        Some(Err(e))      => rsx! { div { class: "task-card", style: "top: 30px; left: 30px;", "Error: {e}" } },
        None              => rsx! { div { class: "task-card", style: "top: 30px; left: 30px;", "Loading…" } },
    };

    rsx! {
        div { class: "layout",
            div { class: "separator1" }
            div { class: "separator2" }
            {card}
        }
    }
}

#[component]
fn TaskCard(task: TaskView) -> Element {
    rsx! {
        div { class: "task-card", style: "top: 30px; left: 30px;",
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
                        SubtaskItem { task: child.clone(), depth: 1 }
                    }
                }
            }
        }
    }
}

#[component]
fn SubtaskItem(task: TaskView, depth: usize) -> Element {
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
            if !task.children.is_empty() {
                ul { class: "subtask-children",
                    for child in &task.children {
                        SubtaskItem { task: child.clone(), depth: depth + 1 }
                    }
                }
            }
        }
    }
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
