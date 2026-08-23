//! E017 / E018 — validates `#MILESTONE` headers project-wide.
//!
//! Unlike every other rule (which only ever looks at a single node, or its
//! direct siblings), this one is inherently project-wide: per README.md,
//! "A milestone name must be provided, and milestones must be unique across
//! the project." `checker::run`/`rules::check_all` are always called with
//! every project file's items already concatenated into one `&[FileItem]`
//! (see `cli::common::parse_files`), so a plain scan over `items` here is
//! naturally project-wide -- no extra file-walking needed, and no need for
//! milestones from different files to be treated any differently than
//! milestones within the same file.

use crate::parser::{FileItem, Milestone};
use crate::rules::{ErrorCode, Issue};
use std::collections::HashMap;

/// Flags every `#MILESTONE` header with no name at all (E018), and every
/// milestone name used more than once across the whole project (E017) --
/// one issue per offending occurrence, mirroring `invalid_order`'s E014
/// "duplicate order number" convention of flagging every occurrence rather
/// than just the second one onward.
///
/// A nameless milestone is never also considered for the duplicate-name
/// check (empty names are excluded from `by_name`): its own "missing name"
/// issue is the more actionable report, and several nameless milestones
/// don't need to *additionally* be reported as duplicates of each other.
pub fn invalid_milestone(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut by_name: HashMap<&str, Vec<&Milestone>> = HashMap::new();

    for item in items {
        let FileItem::Milestone(milestone) = item else {
            continue;
        };
        if milestone.name.is_empty() {
            issues.push(missing_name_issue(milestone));
            continue;
        }
        by_name
            .entry(milestone.name.as_str())
            .or_default()
            .push(milestone);
    }

    for milestones in by_name.values() {
        if milestones.len() < 2 {
            continue;
        }
        for milestone in milestones {
            issues.push(duplicate_name_issue(milestone));
        }
    }

    // Group/dedup order above (HashMap iteration, and interleaving E018s in
    // discovery order) isn't source order; re-sort so output is stable and
    // matches every other rule's file/line ordering.
    issues.sort_by(|a, b| {
        (&a.location.path, a.location.line).cmp(&(&b.location.path, b.location.line))
    });

    issues
}

fn missing_name_issue(milestone: &Milestone) -> Issue {
    Issue {
        location: milestone.location.clone(),
        code: ErrorCode::MissingMilestoneName,
        message: "Milestone has no name".to_string(),
        column: 1,
        help: Some(
            "Provide a name after #MILESTONE, e.g. `#MILESTONE: Release of MVP`.".to_string(),
        ),
        data: None,
    }
}

fn duplicate_name_issue(milestone: &Milestone) -> Issue {
    Issue {
        location: milestone.location.clone(),
        code: ErrorCode::DuplicateMilestoneName,
        message: format!("Duplicate milestone name \"{}\"", milestone.name),
        column: 1,
        help: Some(
            "Milestone names must be unique across the whole project. Rename one of them."
                .to_string(),
        ),
        data: None,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
