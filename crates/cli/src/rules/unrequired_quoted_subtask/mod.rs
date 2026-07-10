//! E011 — flags subtasks that use the `"quoted"` syntax but are not declared
//! as required by any property on their direct parent task or subtask.
//!
//! The `- [ ] "some text"` form is reserved exclusively for property-required
//! subtasks. Using it outside that context is a syntax error.

use crate::config::Config;
use crate::parser::{FileItem, Marker, Subtask, SubtaskKind};
use crate::rules::{ErrorCode, Issue, IssueData};

pub fn unrequired_quoted_subtask(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for item in items {
        if let FileItem::Task(task) = item {
            issues.extend(check_node(&task.markers, &task.children, config));
            for child in &task.children {
                issues.extend(check_subtask_node(child, config));
            }
        }
    }
    issues
}

fn check_subtask_node(subtask: &Subtask, config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    issues.extend(check_node(&subtask.markers, &subtask.children, config));
    for child in &subtask.children {
        issues.extend(check_subtask_node(child, config));
    }
    issues
}

/// For a single node (task or subtask), collect required subtask titles from
/// the node's property markers, then flag any `PropertyRequired` child whose
/// `raw_title` is not in that required set.
fn check_node(markers: &[Marker], children: &[Subtask], config: &Config) -> Vec<Issue> {
    let required = collect_required(markers, config);

    children
        .iter()
        .filter(|c| c.kind == SubtaskKind::PropertyRequired)
        .filter_map(|c| {
            let title = c.raw_title.as_deref()?;
            if required.iter().any(|r| r == title) {
                return None;
            }
            Some(Issue {
                location: c.location.clone(),
                code: ErrorCode::UnrequiredQuotedSubtask,
                message: format!(
                    "Quoted subtask \"{}\" is not declared as a required subtask by any property on the parent",
                    title
                ),
                column: c.indent + 1,
                help: Some(
                    "Remove the surrounding quotes to make this a regular custom subtask, \
                     or declare it as a required subtask in mdagile.toml"
                        .to_string(),
                ),
                data: Some(IssueData::UnrequiredQuotedSubtask {
                    title: title.to_string(),
                }),
            })
        })
        .collect()
}

/// Collects all required subtask titles contributed by the property markers on
/// a node, merging across multiple properties and deduplicating.
fn collect_required(markers: &[Marker], config: &Config) -> Vec<String> {
    let mut required: Vec<String> = Vec::new();
    for marker in markers {
        if let Marker::Property(prop) = marker {
            if let Some(prop_config) = config.properties.get(&prop.name) {
                for s in &prop_config.subtasks {
                    if !required.contains(s) {
                        required.push(s.clone());
                    }
                }
            }
        }
    }
    required
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
