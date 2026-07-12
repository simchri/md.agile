//! E014 / E015 — validates ordered subtasks among siblings.
//!
//! Ordered subtasks (`- [ ] 1. do this`) establish an execution sequence among
//! their direct siblings only — nesting level and parent scope matter, an
//! order number only has meaning relative to the other children of the same
//! parent.

use crate::parser::{FileItem, Order, Status, Subtask};
use crate::rules::{ErrorCode, Issue};

/// Flags duplicate order numbers among siblings (E014) and ordered subtasks
/// marked done while a lower-numbered sibling is still incomplete (E015).
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
    check_duplicate_order_numbers(siblings, issues);
    check_completion_order(siblings, issues);
}

fn check_duplicate_order_numbers(siblings: &[Subtask], issues: &mut Vec<Issue>) {
    for (i, sub) in siblings.iter().enumerate() {
        let Order::Ordered(order_number) = sub.order else {
            continue;
        };
        let has_duplicate = siblings
            .iter()
            .enumerate()
            .any(|(j, other)| j != i && other.order == Order::Ordered(order_number));
        if has_duplicate {
            issues.push(Issue {
                location: sub.location.clone(),
                code: ErrorCode::DuplicateOrderNumber,
                message: format!("Duplicate order number {order_number}"),
                column: 1,
                help: Some(
                    "Another sibling task already uses this order number. Order numbers must be unique among siblings."
                        .to_string(),
                ),
                data: None,
            });
        }
    }
}

fn check_completion_order(siblings: &[Subtask], issues: &mut Vec<Issue>) {
    for sub in siblings {
        let Order::Ordered(order_number) = sub.order else {
            continue;
        };
        if sub.status != Status::Done {
            continue;
        }
        let blocked_by_incomplete_lower_order_number =
            siblings.iter().any(|other| match other.order {
                Order::Ordered(other_order_number) if other_order_number < order_number => {
                    other.status != Status::Done && other.status != Status::Cancelled
                }
                _ => false,
            });
        if blocked_by_incomplete_lower_order_number {
            issues.push(Issue {
                location: sub.location.clone(),
                code: ErrorCode::OutOfOrderCompletion,
                message: "Ordered task completed out of order".to_string(),
                column: 1,
                help: Some(
                    "This task is marked done, but a lower-numbered sibling is still incomplete. \
                     Complete ordered siblings in sequence, or cancel the lower-numbered one."
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
