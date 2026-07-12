//! E014 / E015 — validates ordered subtasks among siblings.
//!
//! Ordered subtasks (`- [ ] 1. do this`) establish an execution sequence among
//! their direct siblings only — nesting level and parent scope matter, an
//! order number only has meaning relative to the other children of the same
//! parent.

use crate::parser::{FileItem, Order, Status, Subtask, SubtaskKind, TASK_LINE_PREFIX_LEN};
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

/// Column of the order-number prefix (e.g. the `1` in `1. do this`) within
/// the source line. Ordinary (`Custom`) subtasks have it as the very first
/// character of the title; `PropertyRequired` subtasks are quoted (e.g.
/// `"1. do this"`), so the prefix sits one character further in, after the
/// opening `"`.
fn order_number_column(sub: &Subtask) -> usize {
    let title_relative_column = match sub.kind {
        SubtaskKind::PropertyRequired => 2,
        SubtaskKind::Custom => 1,
    };
    sub.indent + TASK_LINE_PREFIX_LEN + title_relative_column
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
                column: order_number_column(sub),
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
                column: order_number_column(sub),
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
