use dioxus::prelude::*;
use log::info;

mod server;

use std::collections::{HashMap, HashSet};

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

/// Maximum number of task cards rendered on the canvas at once. The frontend
/// pre-allocates this many `Signal<Option<TaskView>>` slots; any tasks the
/// backend reports beyond the limit are dropped on the GUI side.
const MAX_TASK_SLOTS: usize = 50;

fn app() -> Element {
    let mut tasks_resource = use_resource(|| async { server::get_tasks().await });

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


let mut lowest_rank_done = usize::MAX;
for slot in task_slots.iter() {
    if let Some(task) = slot() {
        if task_progress(&task) >= 1.0 {
            if task.rank < lowest_rank_done {
                lowest_rank_done = task.rank;
            }
        }
    }
}
if lowest_rank_done == usize::MAX {
    lowest_rank_done = 0;
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
                        lowest_rank_done,
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

/// Horizontal step (in px) between two adjacent backlog post-its. The card
/// itself is 110px wide; the extra 10px is the visual gap between cards.
const BACKLOG_OFFSET_PX: usize = 120;
/// The first two slots from the left are reserved for the top-of-backlog
/// post-it, so the rest of the backlog starts two widths in.
const BACKLOG_LEFT_PX: usize = 12 + 0 * BACKLOG_OFFSET_PX;

/// Step (px) and starting offset (px) shared with the backlog row but rendered
/// at the bottom of the canvas. The 12px left inset matches the backlog so
/// the two rows align visually.
const DONE_LEFT_PX: usize = 12;

enum TaskCardState {
    Progress,
    Backlog,
    Done,
}

#[component]
fn TaskCard(task: TaskView, _index: usize,  z_index: usize, on_click: EventHandler<TaskView>, on_hover: EventHandler<TaskView>, lowest_rank_backlog: usize, lowest_rank_done: usize) -> Element {
    let progress = task_progress(&task);

    let z = if z_index > 0 { format!(" z-index: {z_index};") } else { format!(" z-index: 0;") };

    let mut card_style = "task-card".to_string();
    let mut title_style = "task-card-title".to_string();
    let mut markers_style = "task-card-markers".to_string();
    let mut position_style = "".to_string();


    if progress == 0.0 {

        // backlog card style and pos
        card_style = "backlog-card".to_string();
        title_style = "backlog-card-title".to_string();
        markers_style = "backlog-card-markers".to_string();

        let mut pos_index = task.rank;
        if pos_index >= lowest_rank_backlog {
            pos_index = task.rank - lowest_rank_backlog;
        } else {
            pos_index = 0;
        }

        position_style = format!("top: 8px; bottom: unset; left: {}px;{z}", BACKLOG_LEFT_PX + pos_index * BACKLOG_OFFSET_PX);

    } else if progress >= 1.0 {

        // done card style and position
        let mut pos_index = task.rank;
        if pos_index >= lowest_rank_done {
            pos_index = task.rank - lowest_rank_done;
        } else {
            pos_index = 0;
        } 

        // position_style = format!("bottom: 8px; top: unset; left: {}px;{z}", DONE_LEFT_PX + pos_index * BACKLOG_OFFSET_PX);

        let done_top = format!("calc(100vh - {}px)", (110 + 8)); 
        let done_left_px: usize = DONE_LEFT_PX + pos_index * BACKLOG_OFFSET_PX;

        position_style = format!("top: {}; left: {}px;{z}", done_top, done_left_px);

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

/// Returns the completion ratio (0.0..=1.0) used to position the post-it on
/// the diagonal. The top-level checkbox is worth a flat 10% of the total —
/// reserved for the moment the user actually ticks the parent task done — so
/// even a Todo task with every subtask complete tops out at 0.9. Subtasks
/// (counted recursively, with Done and Cancelled treated as complete) fill
/// the remaining 90% proportionally.
fn task_progress(task: &TaskView) -> f64 {
    const PARENT_WEIGHT: f64 = 0.1;
    const SUBTASKS_WEIGHT: f64 = 1.0 - PARENT_WEIGHT;

    if matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled) {
        return 1.0;
    }
    let (done, total) = count_subtasks(task);
    let subtasks_share = if total == 0 {
        0.0
    } else {
        done as f64 / total as f64
    };
    SUBTASKS_WEIGHT * subtasks_share
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
