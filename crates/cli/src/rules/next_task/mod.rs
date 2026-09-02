//! Single source of truth for "what is the next (sub)task to work on" —
//! the eligibility concern shared by `agile task next`'s highlighting walk,
//! `agile task next --mine`'s top-level task selection, and `agile task
//! list --mine`'s filtering. All three ultimately reduce to the same
//! underlying question ([`is_next_task`]) applied at different scopes, so
//! they're defined here once instead of three subtly-diverging copies.
//!
//! **A node is never restricted to being a literal childless leaf.** A node
//! broken down into subtasks is normally worked on *through* those children,
//! not directly — but once every one of its descendants has been resolved
//! (`Done`/`Cancelled`), there's nothing left to delegate to, and the node
//! itself becomes the concrete next actionable unit of work, regardless of
//! how deep it sits in the tree. See [`has_no_remaining_descendant_work`].

use crate::config::Config;
use crate::parser::{Marker, Order, Status, Subtask};
use crate::rules::invalid_order;
use crate::rules::unauthorized_completion;
use crate::rules::{NodeRef, ResolvedIdentity};

/// Returns whether `node`'s own ordering makes it currently unactionable
/// among `siblings`: it carries an [`Order::Ordered`] value and a
/// lower-ordered sibling is still incomplete (not `Done`/`Cancelled`). See
/// [`invalid_order::blocked_by_incomplete_lower_order`], the same check used
/// for E015 "ordered task completed out of order".
fn is_order_blocked(node: NodeRef, siblings: &[Subtask]) -> bool {
    matches!(
        node.order(),
        Some(Order::Ordered(order_number))
            if invalid_order::blocked_by_incomplete_lower_order(siblings, *order_number)
    )
}

/// Returns whether `identity` is eligible for a single node based solely on
/// its own assignment markers (no recursion into children) — the base case
/// for [`is_eligible_for`].
fn is_eligible_by_own_markers(
    markers: &[Marker],
    identity: &ResolvedIdentity,
    config: &Config,
) -> bool {
    let names = unauthorized_completion::assignment_names(markers);
    if names.is_empty() {
        return true;
    }
    let authorized = unauthorized_completion::authorized_users(&names, config);
    match identity {
        ResolvedIdentity::Known(user) => authorized.iter().any(|a| a == user),
        ResolvedIdentity::Unrecognized => false,
    }
}

/// Returns whether every descendant of `node` (not `node` itself) is
/// `Done` or `Cancelled` — i.e. there is no remaining incomplete work
/// anywhere below `node`. When this holds, `node` is the last actionable
/// unit of work in its own subtree — a "leaf" for practical purposes,
/// whether or not it's literally childless — since there's nothing left
/// beneath it to delegate to. Ignores assignment entirely: this is about
/// whether *any* work remains below, for anyone, not just `identity`.
fn has_no_remaining_descendant_work(node: NodeRef) -> bool {
    node.children().iter().all(|child| {
        matches!(child.status, Status::Done | Status::Cancelled)
            && has_no_remaining_descendant_work(NodeRef::Subtask(child))
    })
}

/// The single source of truth for "is `node` the next actionable task" —
/// the concern shared by every command that highlights or selects the next
/// piece of work: `agile task next`'s bolding walk, and `agile task
/// next --mine`/`list`'s task-level filtering both ultimately reduce to this
/// same question for the nodes they consider.
///
/// A node qualifies when all of the following hold:
/// - its status is [`Status::Todo`] (not already `Done`/`Cancelled`);
/// - it isn't [`is_order_blocked`] by an incomplete lower-ordered sibling
///   within `siblings` (the sibling slice `node` was found in — see
///   [`NodeRef::children`] on the parent);
/// - if `identity` is `Some((identity, config))`, it's also eligible by its
///   own assignment markers for that identity — unassigned, or assigned to
///   `identity` (directly or via group membership). Pass `None` for the
///   unconditional case (no `--mine`/`--as`), where any node qualifies
///   regardless of assignment;
/// - it [`has_no_remaining_descendant_work`] — either it has no children at
///   all, or every one of its descendants is already `Done`/`Cancelled`, so
///   there's nothing left below it to work on instead. A node broken down
///   into subtasks that still has incomplete work beneath it never
///   qualifies itself — the actual next task is always one of those
///   descendants (or nothing, if none of them are eligible either).
pub fn is_next_task(
    node: NodeRef,
    siblings: &[Subtask],
    identity: Option<(&ResolvedIdentity, &Config)>,
) -> bool {
    *node.status() == Status::Todo
        && !is_order_blocked(node, siblings)
        && identity
            .map(|(identity, config)| is_eligible_by_own_markers(node.markers(), identity, config))
            .unwrap_or(true)
        && has_no_remaining_descendant_work(node)
}

/// Returns whether `identity` is eligible to work on `node`: `true` if the
/// node carries no `@user`/`@group` assignment markers at all (unassigned
/// tasks are open to anyone, mirroring the E013 `unauthorized_completion`
/// philosophy that assignment never restricts *unassigned* tasks), or if
/// `identity` is directly assigned or a member of an assigned group.
///
/// An explicit assignment on `node` itself is checked *first* and, if it
/// excludes `identity`, blocks the whole subtree — assigning a parent or
/// mid-level task claims everything beneath it, so an unassigned descendant
/// under an explicitly-assigned ancestor is not up for grabs.
///
/// Otherwise, if `node` has been broken down into subtasks, eligibility is
/// recursive: `node` is eligible if at least one actionable (`Todo`, not
/// `Done`/`Cancelled`, not order-blocked — see [`is_order_blocked`]) child is
/// eligible. `#OPT` children still count towards eligibility like any other
/// child. If no such child exists — either because `node` has no children,
/// or because every one of them has already been resolved (`Done`/
/// `Cancelled`; see [`has_no_remaining_descendant_work`]) — `node` itself is
/// the eligible unit: there's nothing left to delegate to, regardless of how
/// deep `node` sits in the tree. A child that's still merely incomplete
/// (whether blocked, or assigned to someone else) does *not* trigger this
/// fallback — there's genuinely remaining work there, just not currently
/// actionable for `identity`, so `node` itself is correctly *not* eligible
/// either in that case.
///
/// Used by `agile task next --mine` (to compute a top-level task's rank
/// among tasks eligible for `identity`) and `agile task list --mine`.
pub fn is_eligible_for(node: NodeRef, identity: &ResolvedIdentity, config: &Config) -> bool {
    if !is_eligible_by_own_markers(node.markers(), identity, config) {
        return false;
    }
    let children = node.children();
    if children.is_empty() {
        return true;
    }
    let has_actionable_eligible_child = children.iter().any(|child| {
        child.status == Status::Todo
            && !is_order_blocked(NodeRef::Subtask(child), children)
            && is_eligible_for(NodeRef::Subtask(child), identity, config)
    });
    has_actionable_eligible_child || has_no_remaining_descendant_work(node)
}

/// Returns whether `node` is a "previous" candidate — the mirror image of
/// [`is_next_task`] for `agile task previous`'s highlighting: it is itself
/// already resolved (`Done`/`Cancelled`) and [`has_no_remaining_descendant_work`],
/// i.e. it's the last concrete unit of completed work in its own subtree,
/// whether or not it's a literal childless leaf. Unlike [`is_next_task`],
/// this ignores ordering and assignment entirely — undoing/reviewing past
/// work isn't gated by either.
pub fn is_previous_task(node: NodeRef) -> bool {
    matches!(node.status(), Status::Done | Status::Cancelled)
        && has_no_remaining_descendant_work(node)
}

/// Returns whether `node` itself, or any descendant of it, is `Done` or
/// `Cancelled` — i.e. `node` has *some* completed work in its subtree, even
/// if not all of it. This is the reverse-rank candidacy rule used by
/// `agile task previous`/the generalized `agile task undone`: a top-level
/// task counts as a candidate the moment any part of it has been touched,
/// not just once it's fully done.
pub fn has_closed_work(node: NodeRef) -> bool {
    matches!(node.status(), Status::Done | Status::Cancelled)
        || node
            .children()
            .iter()
            .any(|child| has_closed_work(NodeRef::Subtask(child)))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
