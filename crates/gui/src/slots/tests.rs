use super::*;
use crate::server::{TaskStatus, TaskView};

fn task(title: &str, rank: usize) -> TaskView {
    TaskView {
        status: TaskStatus::Todo,
        title: title.to_string(),
        markers: vec![],
        body: vec![],
        children: vec![],
        rank,
    }
}

fn done_task(title: &str, rank: usize) -> TaskView {
    TaskView {
        status: TaskStatus::Done,
        ..task(title, rank)
    }
}

fn tasks(slots: &[SlotState]) -> Vec<Option<TaskView>> {
    slots.iter().map(|s| s.task.clone()).collect()
}

fn pool(tasks: &[Option<TaskView>]) -> Vec<SlotState> {
    tasks
        .iter()
        .map(|t| match t {
            Some(task) => SlotState {
                task: Some(task.clone()),
                physics: PhysCard::new(CardPosition { x: 0.0, y: 0.0 }),
            },
            None => SlotState::empty(),
        })
        .collect()
}

#[test]
fn empty_pool_stays_empty_with_no_new_tasks() {
    let current = pool(&[None, None, None]);
    let result = reconcile(&current, &[]);
    assert_eq!(tasks(&result), vec![None, None, None]);
}

#[test]
fn arrivals_fill_empty_slots_in_rank_order() {
    let current = pool(&[None, None, None]);
    let new = vec![task("c", 2), task("a", 0), task("b", 1)];
    let result = reconcile(&current, &new);
    assert_eq!(
        tasks(&result),
        vec![Some(task("a", 0)), Some(task("b", 1)), Some(task("c", 2))]
    );
}

#[test]
fn existing_tasks_keep_their_slot() {
    let current = pool(&[Some(task("a", 0)), Some(task("b", 1))]);
    let new = vec![task("b", 1), task("a", 0)];
    let result = reconcile(&current, &new);
    assert_eq!(tasks(&result), vec![Some(task("a", 0)), Some(task("b", 1))]);
}

#[test]
fn evicted_tasks_become_none() {
    let current = pool(&[Some(task("a", 0)), Some(task("b", 1))]);
    let new = vec![task("a", 0)];
    let result = reconcile(&current, &new);
    assert_eq!(tasks(&result), vec![Some(task("a", 0)), None]);
}

#[test]
fn updated_task_keeps_slot_with_new_fields() {
    let mut original = task("a", 0);
    original.markers = vec!["#X".into()];
    let current = pool(&[Some(original)]);

    let mut updated = task("a", 0);
    updated.markers = vec!["#Y".into()];
    let new = vec![updated.clone()];

    let result = reconcile(&current, &new);
    assert_eq!(tasks(&result), vec![Some(updated)]);
}

#[test]
fn arrivals_fill_only_empty_slots_kept_tasks_remain() {
    let current = pool(&[Some(task("a", 0)), None, Some(task("c", 2))]);
    let new = vec![task("a", 0), task("b", 1), task("c", 2)];
    let result = reconcile(&current, &new);
    assert_eq!(
        tasks(&result),
        vec![Some(task("a", 0)), Some(task("b", 1)), Some(task("c", 2))]
    );
}

#[test]
fn arrivals_beyond_capacity_are_dropped() {
    let current = pool(&[None, None]);
    let new = vec![task("a", 0), task("b", 1), task("c", 2)];
    let result = reconcile(&current, &new);
    assert_eq!(tasks(&result), vec![Some(task("a", 0)), Some(task("b", 1))]);
}

#[test]
fn evict_then_arrive_in_same_tick_uses_lowest_rank_arrival_first() {
    let current = pool(&[Some(task("old", 5)), None]);
    let new = vec![task("new_b", 2), task("new_a", 1)];
    let result = reconcile(&current, &new);
    assert_eq!(
        tasks(&result),
        vec![Some(task("new_a", 1)), Some(task("new_b", 2))]
    );
}

#[test]
fn existing_task_preserves_physics_state() {
    use crate::physics::CardVelocity;
    let mut slot = SlotState::arriving(task("a", 0));
    // Simulate physics having moved the card.
    slot.physics.position = CardPosition { x: 0.3, y: 0.3 };
    slot.physics.velocity = CardVelocity { vx: 0.1, vy: 0.1 };

    let current = vec![slot];
    let new = vec![task("a", 0)];
    let result = reconcile(&current, &new);

    assert_eq!(result[0].physics.position, CardPosition { x: 0.3, y: 0.3 });
    assert_eq!(
        result[0].physics.velocity,
        CardVelocity { vx: 0.1, vy: 0.1 }
    );
}

#[test]
fn new_in_progress_card_starts_at_progress_position() {
    use crate::card_positioning::has_started;
    let current = pool(&[None]);
    // Construct a task that will have progress ~0.5.
    // We need a task with some subtasks done and some not.
    // For simplicity, use a task with markers containing "#STARTED" or similar —
    // actually task_progress is based on children. Let's just verify the
    // arriving() helper sets a non-zero position for a non-backlog task.
    // Use SlotState::arriving() directly.
    let mut t = task("in_prog", 0);
    t.children = vec![
        TaskView {
            status: crate::server::TaskStatus::Done,
            title: "sub1".into(),
            markers: vec![],
            body: vec![],
            children: vec![],
            rank: 0,
        },
        TaskView {
            status: crate::server::TaskStatus::Todo,
            title: "sub2".into(),
            markers: vec![],
            body: vec![],
            children: vec![],
            rank: 0,
        },
    ];
    let _ = current; // not used in this variant
    let slot = SlotState::arriving(t);
    // Progress = 1 done / 2 total, weighted: 0.9 * 0.5 = 0.45.
    assert_eq!(slot.physics.position, CardPosition { x: 0.45, y: 0.45 });
}

#[test]
fn done_task_physics_is_reset_on_reconcile() {
    use crate::physics::CardVelocity;
    let mut slot = SlotState::arriving(task("a", 0));
    slot.physics.position = CardPosition { x: 0.3, y: 0.3 };
    slot.physics.velocity = CardVelocity { vx: 0.1, vy: 0.1 };

    let current = vec![slot];
    let new = vec![done_task("a", 0)];
    let result = reconcile(&current, &new);

    assert_eq!(result[0].physics.position, CardPosition { x: 0.0, y: 0.0 });
    assert_eq!(
        result[0].physics.velocity,
        CardVelocity { vx: 0.0, vy: 0.0 }
    );
}
