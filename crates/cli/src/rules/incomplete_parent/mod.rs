//! E004 — flags done parents with incomplete non-optional children.

use crate::parser::{FileItem, SpecialMarkerKind, Status, Subtask};
use crate::rules::{Issue, for_each_node};

/// Flags tasks/subtasks marked done `[x]` that have incomplete children.
/// A done parent with incomplete children is invalid unless the children are optional (`#OPT`).
/// Cancelled parents are ignored.
pub fn incomplete_parent(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        if *node.status() == Status::Done {
            issues.extend(check_children_complete(
                node.children(),
                node.location(),
                node.indent(),
            ));
        }
    });
    issues
}

/// Flags `children` that would make `parent_location` (at `parent_indent`)
/// an invalid "done" parent: any non-optional child that isn't done or
/// cancelled. Exposed at `pub(crate)` so `agile task done <address>` can
/// reuse this exact rule to check whether marking a node done right now
/// would be valid, without duplicating the logic or running the full
/// `incomplete_parent` rule over an entire project.
pub(crate) fn check_children_complete(
    children: &[Subtask],
    parent_location: &crate::parser::Location,
    parent_indent: usize,
) -> Vec<Issue> {
    let mut issues = Vec::new();

    for child in children {
        // Skip checking optional children - they don't block parent completion
        let is_optional = child.markers.iter().any(
            |m| matches!(m, crate::parser::Marker::Special(s) if s.kind == SpecialMarkerKind::Opt),
        );

        if is_optional {
            continue;
        }

        // If child is not done and not cancelled, parent completion is invalid
        if child.status != Status::Done && child.status != Status::Cancelled {
            issues.push(Issue {
                location: parent_location.clone(),
                code: crate::rules::ErrorCode::IncompleteParent,
                message: "Incomplete parent".to_string(),
                column: parent_indent + 1,
                help: Some(
                    "This task is marked done, but it has incomplete children. \
                     Mark all required children done, cancel them, or make them optional with #OPT."
                        .to_string(),
                ),
                data: None,
            });
            break; // Only report once per parent
        }
    }

    issues
}

#[cfg(test)]
mod tests;
