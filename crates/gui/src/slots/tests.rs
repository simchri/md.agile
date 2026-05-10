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

#[test]
fn empty_pool_stays_empty_with_no_new_tasks() {
    let current = vec![None, None, None];
    let result = reconcile(&current, &[]);
    assert_eq!(result, vec![None, None, None]);
}

#[test]
fn arrivals_fill_empty_slots_in_rank_order() {
    let current = vec![None, None, None];
    let new = vec![task("c", 2), task("a", 0), task("b", 1)];
    let result = reconcile(&current, &new);
    assert_eq!(
        result,
        vec![Some(task("a", 0)), Some(task("b", 1)), Some(task("c", 2))]
    );
}

#[test]
fn existing_tasks_keep_their_slot() {
    let current = vec![Some(task("a", 0)), Some(task("b", 1))];
    let new = vec![task("b", 1), task("a", 0)];
    let result = reconcile(&current, &new);
    assert_eq!(result, vec![Some(task("a", 0)), Some(task("b", 1))]);
}

#[test]
fn evicted_tasks_become_none() {
    let current = vec![Some(task("a", 0)), Some(task("b", 1))];
    let new = vec![task("a", 0)];
    let result = reconcile(&current, &new);
    assert_eq!(result, vec![Some(task("a", 0)), None]);
}

#[test]
fn updated_task_keeps_slot_with_new_fields() {
    let mut original = task("a", 0);
    original.markers = vec!["#X".into()];
    let current = vec![Some(original)];

    let mut updated = task("a", 0);
    updated.markers = vec!["#Y".into()];
    let new = vec![updated.clone()];

    let result = reconcile(&current, &new);
    assert_eq!(result, vec![Some(updated)]);
}

#[test]
fn arrivals_fill_only_empty_slots_kept_tasks_remain() {
    let current = vec![Some(task("a", 0)), None, Some(task("c", 2))];
    let new = vec![task("a", 0), task("b", 1), task("c", 2)];
    let result = reconcile(&current, &new);
    assert_eq!(
        result,
        vec![Some(task("a", 0)), Some(task("b", 1)), Some(task("c", 2))]
    );
}

#[test]
fn arrivals_beyond_capacity_are_dropped() {
    let current = vec![None, None];
    let new = vec![task("a", 0), task("b", 1), task("c", 2)];
    let result = reconcile(&current, &new);
    assert_eq!(result, vec![Some(task("a", 0)), Some(task("b", 1))]);
}

#[test]
fn evict_then_arrive_in_same_tick_uses_lowest_rank_arrival_first() {
    let current = vec![Some(task("old", 5)), None];
    let new = vec![task("new_b", 2), task("new_a", 1)];
    let result = reconcile(&current, &new);
    // "old" is evicted (slot 0 → None), then arrivals fill in rank order.
    // Arrivals are placed into empty slots in slot-index order.
    assert_eq!(result, vec![Some(task("new_a", 1)), Some(task("new_b", 2))]);
}
