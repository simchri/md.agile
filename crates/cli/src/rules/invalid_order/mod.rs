//! E014 / E015 — validates ordered ("ranked") subtasks among siblings.
//!
//! Ranked subtasks (`- [ ] 1. do this`) establish an execution sequence among
//! their direct siblings only — nesting level and parent scope matter, a rank
//! only has meaning relative to the other children of the same parent.

use crate::parser::{FileItem, Order, Status, Subtask};
use crate::rules::{ErrorCode, Issue};

/// Flags duplicate ranks among siblings (E014) and ranked subtasks marked done
/// while a lower-ranked sibling is still incomplete (E015).
pub fn invalid_order(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for item in items {
        if let FileItem::Task(task) = item {
            check_siblings(&task.children, &mut issues);
            recurse(&task.children, &mut issues);
        }
    }

    issues
}

fn recurse(subtasks: &[Subtask], issues: &mut Vec<Issue>) {
    for sub in subtasks {
        check_siblings(&sub.children, issues);
        recurse(&sub.children, issues);
    }
}

/// Runs both checks over a single sibling list (one parent's direct children).
fn check_siblings(siblings: &[Subtask], issues: &mut Vec<Issue>) {
    check_duplicate_ranks(siblings, issues);
    check_completion_order(siblings, issues);
}

fn check_duplicate_ranks(siblings: &[Subtask], issues: &mut Vec<Issue>) {
    for (i, sub) in siblings.iter().enumerate() {
        let Order::Ranked(rank) = sub.order else {
            continue;
        };
        let has_duplicate = siblings
            .iter()
            .enumerate()
            .any(|(j, other)| j != i && other.order == Order::Ranked(rank));
        if has_duplicate {
            issues.push(Issue {
                location: sub.location.clone(),
                code: ErrorCode::DuplicateOrderRank,
                message: format!("Duplicate order rank {rank}"),
                column: 1,
                help: Some(
                    "Another sibling task already uses this rank. Ranks must be unique among siblings."
                        .to_string(),
                ),
                data: None,
            });
        }
    }
}

fn check_completion_order(siblings: &[Subtask], issues: &mut Vec<Issue>) {
    for sub in siblings {
        let Order::Ranked(rank) = sub.order else {
            continue;
        };
        if sub.status != Status::Done {
            continue;
        }
        let blocked_by_incomplete_lower_rank = siblings.iter().any(|other| match other.order {
            Order::Ranked(other_rank) if other_rank < rank => {
                other.status != Status::Done && other.status != Status::Cancelled
            }
            _ => false,
        });
        if blocked_by_incomplete_lower_rank {
            issues.push(Issue {
                location: sub.location.clone(),
                code: ErrorCode::OutOfOrderCompletion,
                message: "Ranked task completed out of order".to_string(),
                column: 1,
                help: Some(
                    "This task is marked done, but a lower-ranked sibling is still incomplete. \
                     Complete ranked siblings in order, or cancel the lower-ranked one."
                        .to_string(),
                ),
                data: None,
            });
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
