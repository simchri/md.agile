use dioxus::prelude::*;
use log::info;

mod card_positioning;
mod physics;
mod server;
mod slots;

use card_positioning::{
    backlog_position_style, in_progress_style, done_position_style, status_box, status_class,
    task_progress,
};
use server::TaskView;

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

/// Maximum number of task cards rendered on the canvas at once. The frontend
/// pre-allocates this many `Signal<Option<TaskView>>` slots; any tasks the
/// backend reports beyond the limit are dropped on the GUI side.
const MAX_TASK_SLOTS: usize = 60;

fn app() -> Element {
    let mut tasks_resource = use_resource(|| async {
        let tasks = server::get_tasks().await;
        tasks
    });

    // One signal per visible task slot. Empty slots hold `None`. Each slot is
    // its own signal so that a change to one task does not invalidate the
    // others. Initialised with use_hook (runs once on mount) so that
    // Signal::new is not called inside an iterator closure, which would
    // violate the Rules of Hooks.
    let task_slots: Vec<Signal<Option<TaskView>>> = use_hook(|| {
        (0..MAX_TASK_SLOTS)
            .map(|_| Signal::new(None::<TaskView>))
            .collect()
    });


    // Physics simulation will be calculated here in the future.
    // For now, all in-progress cards are positioned on the diagonal.

    // Sync the resource into the per-slot signals whenever a fresh task list
    // arrives. Reconciliation is delegated to `slots::reconcile`; we only
    // write a signal when its slot's value actually changed, to keep
    // unrelated cards from re-rendering.
    {
        let task_slots = task_slots.clone();
        use_effect(move || {
            if let Some(Ok(new_list)) = &*tasks_resource.read() {
                let current: Vec<Option<TaskView>> =
                    task_slots.iter().map(|s| s.peek().clone()).collect();
                let next = slots::reconcile(&current, new_list);
                for (slot, new_value) in task_slots.iter().zip(next) {
                    let mut slot = *slot;
                    if *slot.peek() != new_value {
                        slot.set(new_value);
                    }
                }
            }
        });
    }

    let mut modal_task: Signal<Option<TaskView>> = use_signal(|| None);

    use_effect({
        // Clock, frequency 1s.
        // Poll updates from the server side (e.g. update task list).
        // Paused while a task is open in the modal so the card list does not
        // refresh under the user while they are reading it.
        move || {
            dioxus::prelude::spawn(async move {
                log::info!("use_effect: clock START");
                use wasmtimer::tokio::sleep;

                loop {
                    sleep(std::time::Duration::from_millis(1000)).await;
                    if modal_task.peek().is_none() {
                        tasks_resource.restart();
                    }
                }
            });
        }
    });

    let mut front_index: Signal<Option<usize>> = use_signal(|| None);
    let current_front = front_index();

    let mut lowest_rank_backlog = usize::MAX;
    for slot in task_slots.iter() {
        if let Some(task) = slot() {
            if task_progress(&task) == 0.0 {
                if task.rank < lowest_rank_backlog {
                    lowest_rank_backlog = task.rank;
                }
            }
        }
    }
    if lowest_rank_backlog == usize::MAX {
        lowest_rank_backlog = 0;
    }

    let mut highest_rank_done = 0;
    for slot in task_slots.iter() {
        if let Some(task) = slot() {
            if task_progress(&task) >= 1.0 {
                if task.rank > highest_rank_done {
                    highest_rank_done = task.rank;
                }
            }
        }
    }

    rsx! {
        div { class: "layout",
            div { class: "separator1" }
            div { class: "separator2" }

            for (i, slot) in task_slots.iter().enumerate() {
                if let Some(task) = slot() {
                    TaskCard {
                        task,
                        index: i,
                        z_index: if current_front == Some(i) { 100 } else { 0 },
                        on_click: move |t: TaskView| {
                            front_index.set(Some(i));
                            modal_task.set(Some(t));
                        },
                        on_hover: move |_t: TaskView| {
                            front_index.set(Some(i));
                        },
                        lowest_rank_backlog,
                        highest_rank_done,
                    }
                }
            }

            if let Some(task) = modal_task() {
                TaskModal {
                    task: task,
                    on_close: move |_| {
                        modal_task.set(None);
                    },
                }
            }
        }
    }
}

#[component]
fn TaskCard(
    task: TaskView,
    index: usize,
    z_index: usize,
    on_click: EventHandler<TaskView>,
    on_hover: EventHandler<TaskView>,
    lowest_rank_backlog: usize,
    highest_rank_done: usize,
) -> Element {
    let progress = task_progress(&task);

    let z = format!(" z-index: {z_index};");

    let mut card_style = "task-card";
    let mut title_style = "task-card-title";
    let markers_style = "task-card-markers";
    let position_style;

    if progress == 0.0 {
        card_style = "backlog-card";
        title_style = "backlog-card-title";
        position_style = format!(
            "{}{z}",
            backlog_position_style(task.rank, lowest_rank_backlog)
        );
    } else if progress >= 1.0 {
        card_style = "done-card";
        title_style = "done-card-title";
        position_style = format!("{}{z}", done_position_style(task.rank, highest_rank_done));
    } else {
        // In progress — get position from physics module.
        let card = physics::Card { progress: Some(progress) };
        let positions = physics::step(&[card]);
        let pos = positions[0];
        position_style = in_progress_style(pos.x, pos.y);
    }

    let t = task.clone();
    let t2 = task.clone();
    return rsx! {
        div {
            class: "{card_style} smooth-movement",
            style: "{position_style}",
            onclick: move |_| on_click.call(t.clone()),
            onmouseover: move |_| on_hover.call(t2.clone()),
            div { class: "{markers_style}",
                span { class: status_class(&task.status), {status_box(&task.status)} }
                span { class: "{title_style}", "{task.title}" }
            }

            if !task.markers.is_empty() {
                div { class: "{markers_style}",
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
    };
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
