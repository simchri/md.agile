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

// --- Spring-damper physics constants (perpendicular repulsion) ---
const PHYSICS_MS: u64 = 60;
// Two in-progress cards collide when their progress values are within this threshold.
// At 0.18 progress units ≈ 220 px on a typical 1440-wide viewport.
const COLLISION_THRESHOLD: f64 = 0.30;
// Velocity impulse (px/tick) per unit of progress overlap.
// Equilibrium separation ≈ 2 * K_REPEL * COLLISION_THRESHOLD / K_RESTORE.
const K_REPEL: f64 = 16.0;
// Centering spring: pulls each card's perpendicular offset back toward 0.
const K_RESTORE: f64 = 0.03;
// Velocity retention per tick (lower = snappier settle, higher = more drift).
const DAMPING: f64 = 0.60;
// Boundary springs: activate when a card edge is within this many px of the screen edge.
const BOUNDARY_ZONE_PX: f64 = 80.0;
// Velocity impulse per pixel of penetration into the boundary zone.
const K_BOUNDARY: f64 = 0.08;

// Reference viewport dimensions used by the boundary-spring calculations.
// These match the constants embedded in diagonal_style()'s CSS formula, so
// card pixel positions computed here are consistent with where the CSS places them.
const REF_VW: f64 = 1440.0;
const REF_VH: f64 = 900.0;

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
    // others. Initialised with use_hook (runs once on mount) so that
    // Signal::new is not called inside an iterator closure, which would
    // violate the Rules of Hooks.
    let task_slots: Vec<Signal<Option<TaskView>>> = use_hook(|| {
        (0..MAX_TASK_SLOTS)
            .map(|_| Signal::new(None::<TaskView>))
            .collect()
    });

    // Per-slot physics state: perpendicular offset from the diagonal (px) and velocity (px/tick).
    // Passed as signals to TaskCard so only the affected card re-renders on each physics tick.
    let perp_offsets: Vec<Signal<f64>> =
        use_hook(|| (0..MAX_TASK_SLOTS).map(|_| Signal::new(0.0f64)).collect());
    let perp_vels: Vec<Signal<f64>> =
        use_hook(|| (0..MAX_TASK_SLOTS).map(|_| Signal::new(0.0f64)).collect());

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

    use_effect({
        // Physics loop at PHYSICS_MS Hz (60ms), runs purely in the WASM frontend.
        // Applies spring-damper repulsion between in-progress cards that would overlap
        // along the diagonal, spreading them perpendicular to it.
        let task_slots = task_slots.clone();
        let perp_offsets = perp_offsets.clone();
        let perp_vels = perp_vels.clone();
        move || {
            // Vec<Signal<T>> is not Copy, so it cannot be moved from an FnMut closure.
            // Clone inside the body so each invocation owns its own copies.
            let task_slots = task_slots.clone();
            let perp_offsets = perp_offsets.clone();
            let perp_vels = perp_vels.clone();
            dioxus::prelude::spawn(async move {
                use wasmtimer::tokio::sleep;
                loop {
                    sleep(std::time::Duration::from_millis(PHYSICS_MS)).await;

                    // Snapshot: (slot_index, progress, current_perp_offset) for in-progress cards.
                    let active: Vec<(usize, f64, f64)> = task_slots
                        .iter()
                        .enumerate()
                        .filter_map(|(i, slot)| {
                            let t = slot.peek().clone()?;
                            let p = task_progress(&t);
                            (p > 0.0 && p < 1.0).then(|| (i, p, *perp_offsets[i].peek()))
                        })
                        .collect();

                    // Cards that left in-progress get their physics state zeroed.
                    for (i, slot) in task_slots.iter().enumerate() {
                        let p = slot.peek().as_ref().map_or(0.0, |t| task_progress(t));
                        if p <= 0.0 || p >= 1.0 {
                            if *perp_offsets[i].peek() != 0.0 {
                                let mut s = perp_offsets[i];
                                s.set(0.0);
                            }
                            if *perp_vels[i].peek() != 0.0 {
                                let mut s = perp_vels[i];
                                s.set(0.0);
                            }
                        }
                    }

                    // Pairwise repulsion: cards within COLLISION_THRESHOLD progress units
                    // push each other apart in the perpendicular direction.
                    let mut dv = vec![0.0f64; MAX_TASK_SLOTS];
                    for a in 0..active.len() {
                        for b in (a + 1)..active.len() {
                            let (ia, pa, oa) = active[a];
                            let (ib, pb, ob) = active[b];
                            let dp = (pa - pb).abs();
                            if dp < COLLISION_THRESHOLD {
                                let overlap = COLLISION_THRESHOLD - dp;
                                let force = K_REPEL * overlap;
                                if oa <= ob {
                                    dv[ia] -= force;
                                    dv[ib] += force;
                                } else {
                                    dv[ia] += force;
                                    dv[ib] -= force;
                                }
                            }
                        }
                    }

                    // Integrate: repulsion + boundary springs + centering spring + damping.
                    // Signal::set takes &mut self, so copy the Signal (it is Copy) first.
                    let (vw, vh) = (REF_VW, REF_VH);
                    for &(i, p, offset) in &active {
                        let mut v = *perp_vels[i].peek();

                        // Pairwise repulsion (computed above).
                        v += dv[i];

                        // Boundary springs: push back when a card edge enters the zone.
                        // Card position from diagonal_style():
                        //   left = 5 + p*(vw-230) + offset*0.707
                        //   top  = 15vh+5 + p*(70vh-230) - offset*0.707
                        // Positive perp offset → card moves right+up, so:
                        //   right/top boundary → negative impulse
                        //   left/bottom boundary → positive impulse
                        let left = 5.0 + p * (vw - 230.0) + offset * 0.707;
                        let top = 0.15 * vh + 5.0 + p * (0.70 * vh - 230.0) - offset * 0.707;
                        let right = left + 220.0;
                        let bottom = top + 220.0;

                        if left < BOUNDARY_ZONE_PX {
                            v += K_BOUNDARY * (BOUNDARY_ZONE_PX - left);
                        }
                        if right > vw - BOUNDARY_ZONE_PX {
                            v -= K_BOUNDARY * (right - (vw - BOUNDARY_ZONE_PX));
                        }
                        if top < BOUNDARY_ZONE_PX {
                            v -= K_BOUNDARY * (BOUNDARY_ZONE_PX - top);
                        }
                        if bottom > vh - BOUNDARY_ZONE_PX {
                            v += K_BOUNDARY * (bottom - (vh - BOUNDARY_ZONE_PX));
                        }

                        // Centering spring + damping.
                        v -= K_RESTORE * offset;
                        v *= DAMPING;
                        let new_offset = (offset + v).clamp(-300.0, 300.0);
                        let mut vel_sig = perp_vels[i];
                        vel_sig.set(v);
                        if (new_offset - offset).abs() > 0.05 {
                            let mut off_sig = perp_offsets[i];
                            off_sig.set(new_offset);
                        }
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
                        _index: i,
                        z_index: if current_front == Some(i) { 100 } else { 0 },
                        perp_offset_signal: perp_offsets[i],
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
    perp_offset_signal: Signal<f64>,
    on_click: EventHandler<TaskView>,
    on_hover: EventHandler<TaskView>,
    lowest_rank_backlog: usize,
    highest_rank_done: usize,
) -> Element {
    let progress = task_progress(&task);
    let perp_offset = perp_offset_signal();

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

        position_style = format!("top: 5px; bottom: unset; left: {};{z}", left);
    } else if progress >= 1.0 {
        // done card style and position
        let pos_index;
        if task.rank <= highest_rank_done {
            pos_index = highest_rank_done - task.rank;
        } else {
            pos_index = 0;
        }

        let done_top = "calc(85vh + 5px)".to_string();

        let x_px = CARD_GAP_PX + ((pos_index + 1) * (CARD_WIDTH_PX + CARD_GAP_PX));
        let left = format!("calc(100vw - {x_px}px)");

        position_style = format!("top: {}; left: {};{z}", done_top, left);

        card_style = "done-card".to_string();
        title_style = "done-card-title".to_string();
    } else {
        // Else: In Progress style — physics offset applied here
        position_style = diagonal_style(progress, perp_offset);
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
