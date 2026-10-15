//! E020 — flags top-level tasks carrying a property whose `neighbortasks`
//! config requires a quoted sibling task that top-level tasks structurally
//! can never have (see README.vision.md "Neighbor Tasks": "Properties with
//! Neighbor Tasks can not be set at the top level").
//!
//! This check takes precedence over "Missing Neighbor Task" (E019): a
//! top-level task carrying such a property is always invalid regardless of
//! its (nonexistent) quoted siblings, so also reporting "missing neighbor
//! task" there would be redundant and confusing. [`crate::rules::check_all`]
//! runs this rule *before* `neighbor_tasks`, and `neighbor_tasks` itself
//! skips top-level tasks entirely so the two never fire for the same task.

use crate::config::Config;
use crate::parser::FileItem;
use crate::rules::neighbor_tasks::property_names_requiring_neighbors;
use crate::rules::{ErrorCode, Issue};

pub fn neighbor_task_on_top_level(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();

    for item in items {
        if let FileItem::Task(task) = item {
            let properties = property_names_requiring_neighbors(&task.markers, config);
            if properties.is_empty() {
                continue;
            }

            let quoted: Vec<String> = properties.iter().map(|p| format!("#{p}")).collect();
            issues.push(Issue {
                location: task.location.clone(),
                code: ErrorCode::NeighborTaskOnTopLevel,
                message: format!(
                    "Propert{} {} require{} a neighbor task and can't be set on a top-level task",
                    if properties.len() == 1 { "y" } else { "ies" },
                    quoted.join(", "),
                    if properties.len() == 1 { "s" } else { "" },
                ),
                column: task.indent + 1,
                help: Some(
                    "Move this property onto a subtask instead, or remove it: properties \
                     with Neighbor Tasks can't be set on top-level tasks."
                        .to_string(),
                ),
                data: None,
            });
        }
    }

    issues
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
