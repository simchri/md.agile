//! Pure slot-reconciliation logic for the GUI's polling effect.
//!
//! The frontend keeps a fixed-size pool of "card slots", one per visible
//! task. Whenever fresh tasks arrive from the backend, this module decides
//! which slot each task ends up in: tasks that were already on screen stay
//! in the same slot (so they don't visually jump), gone tasks vacate, and
//! new tasks fill empty slots in priority order.

use crate::card_positioning::task_progress;
use crate::physics::{Card as PhysCard, CardPosition, CardVelocity};
use crate::server::{TaskStatus, TaskView};
use std::collections::{HashMap, HashSet};

/// The full state of a single slot: task data plus physics state.
///
/// Merging these into one type means the physics state travels with the task —
/// reconciliation can preserve velocity/position across server updates, and
/// initialize new cards at the correct position immediately.
#[derive(Debug, Clone)]
pub struct SlotState {
    /// The task currently occupying this slot, or `None` if empty.
    pub task: Option<TaskView>,
    /// Physics state for this card (position, velocity, progress target).
    pub physics: PhysCard,
}

impl SlotState {
    /// Empty slot with physics at rest at the top-left corner.
    pub fn empty() -> Self {
        SlotState {
            task: None,
            physics: PhysCard::new(CardPosition { x: 0.0, y: 0.0 }),
        }
    }

    /// New slot for a freshly arriving task, positioned at its progress target.
    fn arriving(task: TaskView) -> Self {
        let p = task_progress(&task).clamp(0.0, 1.0);
        let initial = if p > 0.0 && p < 1.0 {
            CardPosition { x: p, y: p }
        } else {
            CardPosition { x: 0.0, y: 0.0 }
        };
        SlotState {
            task: Some(task),
            physics: PhysCard::new(initial),
        }
    }
}

/// Reconciles a slot pool against an incoming task list.
///
/// `current` is the slot pool's state at the start of the tick (`None` for
/// empty slots). `new_tasks` is the fresh task list from the backend.
///
/// Returns a `Vec` of the same length as `current` containing the desired
/// post-tick contents of each slot. Tasks beyond the pool capacity are
/// dropped silently.
///
/// # Algorithm
/// 1. For each existing slot: if its task's title is still in `new_tasks`,
///    update with the new fields (preserves slot identity); otherwise
///    evict to `None`.
/// 2. New tasks (in `new_tasks` but not previously in any slot) are placed
///    into empty slots in slot-index order, lowest `rank` first.
pub fn reconcile(current: &[Option<TaskView>], new_tasks: &[TaskView]) -> Vec<Option<TaskView>> {
    let new_by_title: HashMap<&str, &TaskView> =
        new_tasks.iter().map(|t| (t.title.as_str(), t)).collect();

    let mut handled: HashSet<&str> = HashSet::new();
    let mut result: Vec<Option<TaskView>> = current
        .iter()
        .map(|slot| match slot {
            Some(cur) => match new_by_title.get(cur.title.as_str()) {
                Some(updated) => {
                    handled.insert(cur.title.as_str());
                    Some((*updated).clone())
                }
                None => None,
            },
            None => None,
        })
        .collect();

    let mut arrivals: Vec<&TaskView> = new_tasks
        .iter()
        .filter(|t| !handled.contains(t.title.as_str()))
        .collect();
    arrivals.sort_by_key(|t| t.rank);
    let mut arrivals = arrivals.into_iter();

    for slot in result.iter_mut() {
        if slot.is_none() {
            match arrivals.next() {
                Some(t) => *slot = Some(t.clone()),
                None => break,
            }
        }
    }

    result
}

#[cfg(test)]
mod tests;
