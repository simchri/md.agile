use dioxus::prelude::*;
use log::info;

mod card_positioning;
mod physics;
mod server;
mod slots;

use card_positioning::{
    backlog_position_style, done_position_style, in_progress_style, status_box, status_class,
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

/// Target frequency for physics simulation: 40 times per second (25ms per frame).
const PHYSICS_FRAME_MS: u64 = 16;

fn app() -> Element {
    let mut tasks_resource = use_resource(|| async {
        let tasks = server::get_tasks().await;
        tasks
    });

    // Task data and physics live in separate signal arrays.
    // Physics signals are passed directly into TaskCard, which subscribes to
    // them internally. This means physics-loop writes only re-render the one
    // affected card — app() is never triggered by physics ticks.
    let task_slots: Vec<Signal<Option<TaskView>>> = use_hook(|| {
        (0..MAX_TASK_SLOTS)
            .map(|_| Signal::new(None::<TaskView>))
            .collect()
    });
    let card_physics: Vec<Signal<physics::Card>> = use_hook(|| {
        (0..MAX_TASK_SLOTS)
            .map(|_| Signal::new(physics::Card::new(physics::CardPosition { x: 0.0, y: 0.0 })))
            .collect()
    });

    // Sync the resource into the per-slot signals whenever a fresh task list
    // arrives. We build a SlotState view for the reconcile function, then
    // write task changes and physics changes to their respective signals
    // independently. Only task_slots writes can trigger app() re-renders.
    {
        let task_slots = task_slots.clone();
        let card_physics = card_physics.clone();
        use_effect(move || {
            if let Some(Ok(new_list)) = &*tasks_resource.read() {
                let current: Vec<slots::SlotState> = task_slots
                    .iter()
                    .zip(card_physics.iter())
                    .map(|(t, p)| slots::SlotState {
                        task: t.peek().clone(),
                        physics: *p.peek(),
                    })
                    .collect();
                let next = slots::reconcile(&current, new_list);
                for (i, new_value) in next.into_iter().enumerate() {
                    let mut task_sig = task_slots[i];
                    let mut phys_sig = card_physics[i];
                    if *task_sig.peek() != new_value.task {
                        task_sig.set(new_value.task);
                    }
                    if *phys_sig.peek() != new_value.physics {
                        phys_sig.set(new_value.physics);
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

    // Physics loop at PHYSICS_FRAME_MS Hz. Reads task progress from task_slots
    // (via peek — non-reactive) and reads/writes only card_physics signals.
    // Never touches task_slots, so physics ticks never cause app() to re-render.
    {
        let task_slots = task_slots.clone();
        let card_physics = card_physics.clone();
        use_effect(move || {
            let task_slots = task_slots.clone();
            let card_physics = card_physics.clone();
            dioxus::prelude::spawn(async move {
                use wasmtimer::tokio::sleep;
                let dt = PHYSICS_FRAME_MS as f64 / 1000.0;

                loop {
                    sleep(std::time::Duration::from_millis(PHYSICS_FRAME_MS)).await;

                    // Extract physics cards, inject current progress.
                    let mut cards: Vec<physics::Card> = task_slots
                        .iter()
                        .zip(card_physics.iter())
                        .map(|(task_sig, phys_sig)| {
                            let mut card = *phys_sig.peek();
                            card.progress = task_sig
                                .peek()
                                .as_ref()
                                .map(task_progress)
                                .filter(|p| *p > 0.0 && *p < 1.0);
                            card
                        })
                        .collect();

                    physics::step(&mut cards, dt);

                    // Write updated physics back; skip if position unchanged.
                    for (new_card, phys_sig) in cards.into_iter().zip(card_physics.iter()) {
                        let mut phys_sig = *phys_sig;
                        if phys_sig.peek().position != new_card.position {
                            phys_sig.set(new_card);
                        }
                    }
                }
            });
        });
    }

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

            for (i, task_slot) in task_slots.iter().enumerate() {
                if let Some(task) = task_slot() {
                    TaskCard {
                        task,
                        physics_signal: card_physics[i],
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
    physics_signal: Signal<physics::Card>,
    z_index: usize,
    on_click: EventHandler<TaskView>,
    on_hover: EventHandler<TaskView>,
    lowest_rank_backlog: usize,
    highest_rank_done: usize,
) -> Element {
    let progress = task_progress(&task);
    // Subscribe this component directly to the physics signal.
    // Physics-loop writes trigger only this card's re-render, not app().
    let position = physics_signal().position;

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
        // In progress — use position from physics loop.
        position_style = format!("{}{z}", in_progress_style(position.x, position.y));
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
