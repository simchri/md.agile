use dioxus::prelude::*;
use log::info;

mod card_positioning;
mod physics;
mod server;
mod slots;

use card_positioning::{
    backlog_position_style, card_position_from_px, card_top_left_px, done_position_style,
    in_progress_style, status_box, status_class, task_progress,
};
use dioxus::prelude::ServerFnError;
use server::TaskView;

fn main() {
    init_logger();
    info!("mdagile-gui main");

    #[cfg(feature = "server")]
    {
        // `fullstack_address_or_localhost` is where *this* process's own axum
        // server actually binds (via `dioxus::launch` below) — driven by the
        // `PORT`/`IP` env vars, defaulting to 127.0.0.1:8080 if unset.
        //
        // When running under `dx serve`, that is *not* the address a browser
        // should be pointed at: `dx serve` runs its own devserver/proxy in
        // front of this backend, listening on whatever `--port`/`--addr` (or
        // `DIOXUS_DEVSERVER_PORT`/`_IP`) were given, and this backend itself
        // is bound to a separate, often OS-assigned ephemeral port that only
        // the devserver talks to directly. So we log both, labelled, so it's
        // unambiguous which one to actually open in a browser.
        let backend_addr = dioxus_cli_config::fullstack_address_or_localhost();
        if dioxus_cli_config::is_cli_enabled() {
            match dioxus_cli_config::devserver_raw_addr() {
                Some(devserver_addr) => {
                    info!(
                        "running under `dx serve`: browser-facing devserver at http://{devserver_addr} (this is what --port/--addr controls); internal backend bound to http://{backend_addr} (not meant to be opened directly)"
                    );
                    println!(
                        "Open in your browser: http://{devserver_addr}  (internal backend: http://{backend_addr})"
                    );
                }
                None => {
                    // `is_cli_enabled()` was true but no devserver address was
                    // published — fall back to the backend address, flagging
                    // the assumption so this isn't silently misleading.
                    info!(
                        "running under `dx` but no devserver address was found; assuming the backend address is also the browser-facing one: http://{backend_addr}"
                    );
                    println!("Server running on http://{backend_addr}");
                }
            }
        } else {
            // Not running under the `dx` CLI (e.g. `cargo run`, or a shipped
            // binary) — there is no separate devserver/proxy, so this really
            // is the address a client should connect to.
            info!("server starting on http://{backend_addr}");
            println!("Server running on http://{backend_addr}");
        }
    }

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
const PHYSICS_FRAME_MS: u64 = 50;

/// State for an in-progress card currently being dragged by the mouse.
///
/// `click_offset_px` is the offset between where the user clicked and the
/// card's own top-left corner at drag start, so the card doesn't jump to
/// snap its corner under the cursor — it keeps the same relative grab
/// point for the whole gesture.
#[derive(Clone, Copy, PartialEq)]
struct DragState {
    slot_index: usize,
    click_offset_px: (f64, f64),
}

/// Derives whether write actions (e.g. the "Mark done" button) should be
/// hidden, from the current state of the `get_kiosk_mode` resource.
///
/// Defaults to `true` (hide write actions) while the fetch is still in
/// flight (`None`) or if it failed (`Some(Err(_))`) — fail safe, so a slow
/// or broken kiosk-mode check never flashes a write-capable UI. Only an
/// explicit, successful `Ok(false)` response ("kiosk mode is off") flips
/// this to `false` (show write actions). An explicit `Ok(true)` keeps it
/// `true`, as expected.
fn resolve_kiosk_flag(kiosk_mode: Option<&Result<bool, ServerFnError>>) -> bool {
    !matches!(kiosk_mode, Some(Ok(false)))
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

fn app() -> Element {
    let mut tasks_resource = use_resource(|| async {
        let tasks = server::get_tasks().await;
        tasks
    });

    // Fetched once and effectively static for the lifetime of the running
    // server — kiosk mode is a startup-time configuration, not something
    // that changes while the GUI is open. Defaults to hiding write actions
    // (`true`) while the fetch is in flight, to avoid a flash of a "mark
    // done" button that then disappears.
    let kiosk_resource = use_resource(|| async { server::get_kiosk_mode().await });
    let kiosk = resolve_kiosk_flag(kiosk_resource.read().as_ref());

    // Drag-and-drop for in-progress cards. `layout_rect_px` is the pixel
    // bounding box of the `div.layout` container (origin + size), captured
    // once on mount via `onmounted`/`get_client_rect` — needed to convert
    // the mouse's viewport-relative coordinates into coordinates relative
    // to the canvas, matching what `card_top_left_px` expects. `dragging`
    // is only written on mousedown/mouseup (not on every mousemove), so
    // reading it reactively to compute each card's `is_dragging` flag below
    // only re-renders `app()` twice per drag gesture.
    let mut layout_rect_px: Signal<Option<(f64, f64, f64, f64)>> = use_signal(|| None);
    let mut dragging: Signal<Option<DragState>> = use_signal(|| None);

    // Set once real mouse movement is observed during a drag, so the click
    // event that naturally follows the mouseup at the end of a drag gesture
    // doesn't also open the task modal — a drag should only move the card,
    // never count as a "click" on it.
    let mut drag_moved: Signal<bool> = use_signal(|| false);

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
    let card_physics_c = card_physics.clone();

    // Sync the resource into the per-slot signals whenever a fresh task list
    // arrives. We build a SlotState view for the reconcile function, then
    // write task changes and physics changes to their respective signals
    // independently. Only task_slots writes can trigger app() re-renders.
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

                // if a task is "done" Reset the respective physics entry, so a new task using the same card, does not start with the old cards physics state
                // (in the UI shows as card starting jumping to the center, then sliding back to its actual position)
                // I guess this is necesseary, because we have two independent clocks, therefore it is not guaranteed that a physics update runs before the next re-render
                for (slot, phys_sig) in task_slots.iter().zip(card_physics_c.iter()) {
                    if let Some(task) = slot.peek().as_ref() {
                        if task_progress(&task) >= 1.0 {
                            let mut phys_sig = *phys_sig;
                            phys_sig
                                .set(physics::Card::new(physics::CardPosition { x: 0.0, y: 0.0 }));
                        }
                    }
                }
            }
        });
    }

    let mut modal_task: Signal<Option<TaskView>> = use_signal(|| None);

    // While a task is open in the modal, keep it in sync with the latest
    // fetched task list (e.g. after toggling one of its subtasks done/todo
    // via its checkbox) — matched by path+line, since that uniquely
    // identifies the top-level task the modal is showing. This does not
    // close the modal; only toggling the top-level task itself does that
    // (see `on_marked_done` below).
    //
    // `tasks_resource.read()` is called unconditionally (not nested inside
    // the `if let Some(current) = ...` below) so this effect always
    // subscribes to it and re-runs on every fetch — including the very
    // first run, when no modal is open yet and the inner branch is never
    // taken. Reading it only conditionally would mean this effect never
    // subscribes at all until a modal happens to be open on the same run
    // that also reads the resource, i.e. it would silently never re-run.
    use_effect(move || {
        let tasks = tasks_resource.read();
        if let Some(Ok(list)) = &*tasks {
            let current = modal_task.peek().clone();
            if let Some(current) = current {
                if let Some(updated) = list
                    .iter()
                    .find(|t| t.path == current.path && t.line == current.line)
                {
                    if *updated != current {
                        modal_task.set(Some(updated.clone()));
                    }
                }
            }
        }
    });

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

                    // A card currently held by the mouse has its position
                    // driven directly by the drag handlers, not by the
                    // spring simulation below — but it still needs to be
                    // included as an input to `physics::step` so other
                    // cards keep reacting to its (live, dragged) position.
                    let dragged_slot = dragging.peek().as_ref().map(|d| d.slot_index);

                    // update progress value in cards
                    let mut cards: Vec<physics::Card> = task_slots
                        .iter()
                        .zip(card_physics.iter())
                        .map(|(task_sig, phys_sig)| {
                            let mut card = *phys_sig.peek();
                            let new_progress = task_sig
                                .peek()
                                .as_ref()
                                .map(task_progress)
                                .filter(|p| *p > 0.0 && *p < 1.0);

                            // Break x==y symmetry on first activation.
                            // Without this, spring target (p,p) and equal initial
                            // conditions guarantee x==y forever, so all cards are
                            // locked to the diagonal regardless of repulsion forces.
                            if card.progress.is_none() {
                                if let Some(p) = new_progress {
                                    use rand::RngExt;
                                    let offset: f64 = rand::rng().random_range(-0.15_f64..0.15_f64);
                                    card.position = physics::CardPosition {
                                        x: p,
                                        y: p + offset,
                                    };
                                }
                            }

                            card.progress = new_progress;
                            card
                        })
                        .collect();

                    physics::step(&mut cards, dt);

                    // write updated physics properties to signals — except
                    // for the dragged card, whose position/velocity is
                    // owned by the drag handlers for the duration of the
                    // gesture; overwriting it here would fight the mouse.
                    for (idx, (new_card, phys_sig)) in
                        cards.into_iter().zip(card_physics.iter()).enumerate()
                    {
                        if Some(idx) == dragged_slot {
                            continue;
                        }
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

    let dragging_slot = dragging().map(|d| d.slot_index);

    rsx! {
        div {
            class: "layout",
            // Captures the canvas's own pixel bounding box once mounted, so
            // mouse coordinates (viewport-relative) can be translated into
            // canvas-relative pixels matching `card_top_left_px`.
            onmounted: move |evt: dioxus::events::MountedEvent| {
                dioxus::prelude::spawn(async move {
                    if let Ok(rect) = evt.get_client_rect().await {
                        layout_rect_px
                            .set(
                                Some((
                                    rect.origin.x,
                                    rect.origin.y,
                                    rect.size.width,
                                    rect.size.height,
                                )),
                            );
                    }
                });
            },
            onmousemove: move |evt: MouseEvent| {
                if let Some(drag) = dragging() {
                    if let Some((rect_x, rect_y, w, h)) = *layout_rect_px.peek() {
                        drag_moved.set(true);
                        let client = evt.client_coordinates();
                        let mouse_x = client.x - rect_x;
                        let mouse_y = client.y - rect_y;
                        let card_left = mouse_x - drag.click_offset_px.0;
                        let card_top = mouse_y - drag.click_offset_px.1;
                        let (x, y) = card_position_from_px(card_left, card_top, w, h);
                        let mut phys_sig = card_physics[drag.slot_index];
                        let mut card = *phys_sig.peek();
                        card.position = physics::CardPosition { x, y };
                        card.velocity = physics::CardVelocity { vx: 0.0, vy: 0.0 };
                        phys_sig.set(card);
                    }
                }
            },
            onmouseup: move |_evt| dragging.set(None),
            onmouseleave: move |_evt| dragging.set(None),
            div { class: "separator1" }
            div { class: "separator2" }

            for (i, task_slot) in task_slots.iter().enumerate() {
                if let Some(task) = task_slot() {
                    TaskCard {
                        task,
                        physics_signal: card_physics[i],
                        z_index: if current_front == Some(i) { 100 } else { 0 },
                        is_dragging: dragging_slot == Some(i),
                        layout_rect_px,
                        on_click: move |t: TaskView| {
                            if *drag_moved.peek() {
                                drag_moved.set(false);
                                return;
                            }
                            front_index.set(Some(i));
                            modal_task.set(Some(t));
                        },
                        on_hover: move |_t: TaskView| {
                            front_index.set(Some(i));
                        },
                        on_drag_start: move |click_offset_px: (f64, f64)| {
                            dragging.set(Some(DragState { slot_index: i, click_offset_px }));
                        },
                        lowest_rank_backlog,
                        highest_rank_done,
                    }
                }
            }

            if let Some(task) = modal_task() {
                TaskModal {
                    task: task,
                    kiosk,
                    on_close: move |_| {
                        modal_task.set(None);
                    },
                    on_marked_done: move |top_level_done: bool| {
                        if top_level_done {
                            modal_task.set(None);
                        }
                        tasks_resource.restart();
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
    is_dragging: bool,
    layout_rect_px: Signal<Option<(f64, f64, f64, f64)>>,
    on_click: EventHandler<TaskView>,
    on_hover: EventHandler<TaskView>,
    on_drag_start: EventHandler<(f64, f64)>,
    lowest_rank_backlog: usize,
    highest_rank_done: usize,
) -> Element {
    let progress = task_progress(&task);
    // Subscribe this component directly to the physics signal.
    // Physics-loop writes trigger only this card's re-render, not app().
    let position = physics_signal().position;
    let is_in_progress = progress > 0.0 && progress < 1.0;

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

    // While being dragged, movement must track the cursor exactly — the
    // CSS transition otherwise used to smooth ordinary physics motion would
    // just make the card visibly lag behind the mouse.
    let movement_class = if is_dragging { "" } else { " smooth-movement" };

    let t = task.clone();
    let t2 = task.clone();
    return rsx! {
        div {
            class: "{card_style}{movement_class}",
            style: "{position_style}",
            onclick: move |_| on_click.call(t.clone()),
            onmouseover: move |_| on_hover.call(t2.clone()),
            onmousedown: move |evt: MouseEvent| {
                if is_in_progress {
                    if let Some((rect_x, rect_y, w, h)) = *layout_rect_px.peek() {
                        let client = evt.client_coordinates();
                        let mouse_x = client.x - rect_x;
                        let mouse_y = client.y - rect_y;
                        let (card_left, card_top) = card_top_left_px(position.x, position.y, w, h);
                        on_drag_start.call((mouse_x - card_left, mouse_y - card_top));
                    }
                }
            },
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
fn SubtaskItem(
    task: TaskView,
    depth: usize,
    show_body: bool,
    // `bool` is `true` when the task being toggled is currently *done*
    // (so the handler should revert it to todo) and `false` when it's
    // currently *todo* (so the handler should mark it done).
    on_toggle: Option<EventHandler<(String, usize, bool)>>,
    #[props(default)] toggle_disabled: bool,
) -> Element {
    let style = format!("padding-left: {}px;", (depth - 1) * 8);
    let is_todo = matches!(task.status, server::TaskStatus::Todo);
    let is_done = matches!(task.status, server::TaskStatus::Done);
    let clickable = on_toggle.is_some() && (is_todo || is_done) && !toggle_disabled;
    let path = task.path.clone();
    let line = task.line;
    rsx! {
        li { class: "subtask {status_class(&task.status)}", style: "{style}",
            span {
                class: if clickable { "subtask-status clickable" } else { "subtask-status" },
                onclick: move |evt: MouseEvent| {
                    if clickable {
                        evt.stop_propagation();
                        if let Some(on_toggle) = on_toggle {
                            on_toggle.call((path.clone(), line, is_done));
                        }
                    }
                },
                {status_box(&task.status)}
            }
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
                        SubtaskItem {
                            task: child.clone(),
                            depth: depth + 1,
                            show_body: show_body,
                            on_toggle: on_toggle,
                            toggle_disabled: toggle_disabled,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TaskModal(
    task: TaskView,
    kiosk: bool,
    on_close: EventHandler<MouseEvent>,
    // `bool` argument is `true` when the *top-level* task shown by this
    // modal was just marked done or reverted to todo (so the caller should
    // close the modal); `false` when one of its subtasks was toggled
    // instead (the modal should stay open, showing the now-updated subtask
    // list).
    on_marked_done: EventHandler<bool>,
) -> Element {
    let mut error_text: Signal<String> = use_signal(String::new);
    let mut error_show: Signal<bool> = use_signal(|| false);
    let mut pending = use_signal(|| false);

    let is_todo = matches!(task.status, server::TaskStatus::Todo);
    let is_done = matches!(task.status, server::TaskStatus::Done);
    let path = task.path.clone();
    let line = task.line;

    // Marks the given (path, line) task done, disabling all checkboxes
    // while the request is in flight and surfacing any failure as a
    // snackbar. `is_top_level` is forwarded to `on_marked_done` unchanged,
    // so the caller only closes the modal when the top-level task itself —
    // not a subtask — was marked done.
    let mark_done = EventHandler::new(move |(path, line, is_top_level): (String, usize, bool)| {
        pending.set(true);
        error_show.set(false);
        dioxus::prelude::spawn(async move {
            match server::mark_task_done(path, line).await {
                Ok(()) => {
                    // Reset even though the modal closes for the top-level
                    // case — for a subtask, the modal stays open and this
                    // is what re-enables the other checkboxes.
                    pending.set(false);
                    on_marked_done.call(is_top_level);
                }
                Err(e) => {
                    pending.set(false);
                    error_text.set(e.to_string());
                    error_show.set(true);
                }
            }
        });
    });

    // Reverts the given (path, line) task to todo — the same
    // pending/error/close-modal-only-for-top-level dance as `mark_done`,
    // just calling the inverse server action.
    let mark_undone =
        EventHandler::new(move |(path, line, is_top_level): (String, usize, bool)| {
            pending.set(true);
            error_show.set(false);
            dioxus::prelude::spawn(async move {
                match server::mark_task_undone(path, line).await {
                    Ok(()) => {
                        pending.set(false);
                        on_marked_done.call(is_top_level);
                    }
                    Err(e) => {
                        pending.set(false);
                        error_text.set(e.to_string());
                        error_show.set(true);
                    }
                }
            });
        });

    // Subtask checkboxes always report `is_top_level: false` — only the
    // header checkbox above can close the modal. `was_done` picks which
    // direction to toggle: a currently-done subtask is reverted to todo,
    // a currently-todo one is marked done.
    let mark_subtask_toggle =
        EventHandler::new(move |(path, line, was_done): (String, usize, bool)| {
            if was_done {
                mark_undone.call((path, line, false));
            } else {
                mark_done.call((path, line, false));
            }
        });

    // Checkboxes are clickable only outside kiosk mode, while no request is
    // pending, and only for tasks that are still Todo or Done (cancelled
    // tasks have no toggle action defined).
    let header_clickable = !kiosk && (is_todo || is_done) && !pending();

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
                    span {
                        class: if header_clickable { "modal-status {status_class(&task.status)} clickable" } else { "modal-status {status_class(&task.status)}" },
                        onclick: move |evt: MouseEvent| {
                            if header_clickable {
                                evt.stop_propagation();
                                if is_done {
                                    mark_undone.call((path.clone(), line, true));
                                } else {
                                    mark_done.call((path.clone(), line, true));
                                }
                            }
                        },
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
                            SubtaskItem {
                                task: child.clone(),
                                depth: 1,
                                show_body: true,
                                on_toggle: mark_subtask_toggle,
                                toggle_disabled: kiosk || pending(),
                            }
                        }
                    }
                }

                Snackbar { text: error_text, show: error_show }
            }
        }
    }
}

/// Duration a [`Snackbar`] stays visible after `show` is set to `true`,
/// in milliseconds.
const SNACKBAR_DISPLAY_MS: u32 = 3500;

/// A transient, auto-dismissing notification banner — used to surface
/// errors (e.g. a rejected mark-done/undone request) without a modal
/// dialog the user has to explicitly close. Ported from the equivalent
/// component in the `buckett` project.
///
/// `show` is set to `true` by the caller to (re-)display `text`; this
/// component takes ownership of setting it back to `false` once
/// [`SNACKBAR_DISPLAY_MS`] has elapsed. Setting `show` to `true` again
/// while already visible restarts the display timer.
#[component]
fn Snackbar(text: Signal<String>, mut show: Signal<bool>) -> Element {
    let mut remaining_ms = use_signal(|| 0u32);

    use_effect(move || {
        if show() {
            remaining_ms.set(SNACKBAR_DISPLAY_MS);
        }
    });

    // Runs once (see the `dx serve` clock effect above for the same
    // pattern): a persistent background countdown loop, driven by
    // `remaining_ms` rather than by effect reactivity.
    use_effect(move || {
        dioxus::prelude::spawn(async move {
            use wasmtimer::tokio::sleep;
            loop {
                if remaining_ms() > 0 {
                    sleep(std::time::Duration::from_millis(100)).await;
                    let t = remaining_ms();
                    if t <= 100 {
                        remaining_ms.set(0);
                        show.set(false);
                    } else {
                        remaining_ms.set(t - 100);
                    }
                } else {
                    sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        });
    });

    if !show() {
        return rsx! {};
    }

    rsx! {
        div { class: "snackbar", "{text}" }
    }
}
