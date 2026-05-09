use dioxus::prelude::*;
use log::info;

mod card_positioning;
mod server;

use std::collections::{HashMap, HashSet};

use card_positioning::{diagonal_style, status_box, status_class, task_progress};
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
const MAX_TASK_SLOTS: usize = 50;

fn app() -> Element {
    let mut tasks_resource = use_resource(|| async {
        let tasks = server::get_tasks().await;
        let _num_tasks = match &tasks {
            Ok(t) => t.len(),
            Err(e) => {
                log::error!("error fetching tasks: {e}");
                0
            }
        };
        tasks
    });

    // One signal per visible task slot. Empty slots hold `None`. Each slot is
    // its own signal so that a change to one task does not invalidate the
    // others.
    let task_slots: Vec<Signal<Option<TaskView>>> = (0..MAX_TASK_SLOTS)
        .map(|_| use_signal(|| None::<TaskView>))
        .collect();

    // Sync the resource into the per-slot signals whenever a fresh task list
    // arrives. Matching is by title so the slot a task occupies is stable
    // across polls: unchanged tasks stay in place (no signal write, no
    // re-render), gone tasks vacate their slot, and new tasks fill any
    // available empty slot.
    {
        let task_slots = task_slots.clone();
        use_effect(move || {
            if let Some(Ok(new_list)) = &*tasks_resource.read() {
                let new_by_title: HashMap<String, TaskView> = new_list
                    .iter()
                    .map(|t| (t.title.clone(), t.clone()))
                    .collect();

                let mut handled: HashSet<String> = HashSet::new();

                // Pass 1: update in-place or evict tasks whose title is gone.
                for slot in &task_slots {
                    let mut slot = *slot;
                    let current: Option<TaskView> = (*slot.peek()).clone();
                    if let Some(cur) = current {
                        if let Some(updated) = new_by_title.get(&cur.title) {
                            if &cur != updated {
                                slot.set(Some(updated.clone()));
                            }
                            handled.insert(cur.title);
                        } else {
                            slot.set(None);
                        }
                    }
                }

                // Pass 2: place arriving tasks into empty slots, lowest rank first.
                let mut arrivals: Vec<&TaskView> = new_by_title
                    .values()
                    .filter(|t| !handled.contains(&t.title))
                    .collect();
                arrivals.sort_by_key(|t| t.rank);
                let mut arrivals = arrivals.into_iter();

                for slot in &task_slots {
                    let mut slot = *slot;
                    if slot.peek().is_none() {
                        match arrivals.next() {
                            Some(task) => slot.set(Some(task.clone())),
                            None => break,
                        }
                    }
                }
            }
        });
    }

    use_effect({
        // Clock, frequency 1s.
        // Poll updates from the server side (e.g. update task list)
        move || {
            dioxus::prelude::spawn(async move {
                log::info!("use_effect: clock START");
                use wasmtimer::tokio::sleep;

                loop {
                    sleep(std::time::Duration::from_millis(1000)).await;
                    tasks_resource.restart();
                }
            });
        }
    });

    let mut modal_task: Signal<Option<TaskView>> = use_signal(|| None);
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
                        _index: i,
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
    _index: usize,
    z_index: usize,
    on_click: EventHandler<TaskView>,
    on_hover: EventHandler<TaskView>,
    lowest_rank_backlog: usize,
    highest_rank_done: usize,
) -> Element {
    let progress = task_progress(&task);

    let z = if z_index > 0 {
        format!(" z-index: {z_index};")
    } else {
        format!(" z-index: 0;")
    };

    let mut card_style = "task-card".to_string();
    let mut title_style = "task-card-title".to_string();
    let markers_style = "task-card-markers".to_string();
    let position_style;

    let CARD_WIDTH_PX = 110;
    let CARD_GAP_PX = 8;

    if progress == 0.0 {
        // backlog card style and pos
        card_style = "backlog-card".to_string();
        title_style = "backlog-card-title".to_string();

        let mut pos_index = task.rank;
        if pos_index >= lowest_rank_backlog {
            pos_index = task.rank - lowest_rank_backlog;
        } else {
            pos_index = 0;
        }

        let x_px = CARD_GAP_PX + ((pos_index) * (CARD_WIDTH_PX + CARD_GAP_PX));
        let left = format!("{x_px}px");

        position_style = format!("top: 8px; bottom: unset; left: {};{z}", left);
    } else if progress >= 1.0 {
        // done card style and position
        let pos_index;
        if task.rank <= highest_rank_done {
            pos_index = highest_rank_done - task.rank;
        } else {
            pos_index = 0;
        }

        log::info!("task: {}", task.title);
        log::info!(
            "rank: {}, lowest_rank_done: {}, pos_index: {}",
            task.rank,
            highest_rank_done,
            pos_index
        );

        let done_top = format!("calc(100vh - {}px)", (110 + 8));

        let x_px = CARD_GAP_PX + ((pos_index + 1) * (CARD_WIDTH_PX + CARD_GAP_PX));
        let left = format!("calc(100vw - {x_px}px)");

        position_style = format!("top: {}; left: {};{z}", done_top, left);

        card_style = "done-card".to_string();
        title_style = "done-card-title".to_string();
    } else {
        // Else: In Progress style
        position_style = diagonal_style(progress);
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
