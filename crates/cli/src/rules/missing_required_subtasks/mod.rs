//! E010 — flags tasks/subtasks that are missing required subtasks mandated by a property.

use crate::config::Config;
use crate::parser::Location;
use crate::parser::{FileItem, Marker, Subtask, SubtaskKind};
use crate::rules::{ErrorCode, Issue, IssueData};

pub fn missing_required_subtasks(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for item in items {
        if let FileItem::Task(task) = item {
            issues.extend(check_node(
                &task.markers,
                &task.children,
                &task.location,
                config,
            ));
            for child in &task.children {
                issues.extend(check_subtask_node(child, config));
            }
        }
    }
    issues
}

fn check_subtask_node(subtask: &Subtask, config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    issues.extend(check_node(
        &subtask.markers,
        &subtask.children,
        &subtask.location,
        config,
    ));
    for child in &subtask.children {
        issues.extend(check_subtask_node(child, config));
    }
    issues
}

fn check_node(
    markers: &[Marker],
    children: &[Subtask],
    location: &Location,
    config: &Config,
) -> Vec<Issue> {
    // Collect all required subtask strings from every property marker on this node,
    // merging across multiple properties and deduplicating.
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

    if required.is_empty() {
        return vec![];
    }

    // Collect raw_title strings of all PropertyRequired direct children.
    let present: Vec<&str> = children
        .iter()
        .filter(|c| c.kind == SubtaskKind::PropertyRequired)
        .filter_map(|c| c.raw_title.as_deref())
        .collect();

    let missing: Vec<String> = required
        .iter()
        .filter(|r| !present.contains(&r.as_str()))
        .cloned()
        .collect();

    if missing.is_empty() {
        return vec![];
    }

    let missing_quoted: Vec<String> = missing.iter().map(|s| format!("\"{s}\"")).collect();
    vec![Issue {
        location: location.clone(),
        code: ErrorCode::MissingRequiredSubtasks,
        message: format!(
            "Missing required subtask{}: {}",
            if missing.len() == 1 { "" } else { "s" },
            missing_quoted.join(", "),
        ),
        column: 1,
        help: Some("Add the missing quoted subtask(s), e.g. `- [ ] \"PO review\"`".to_string()),
        data: Some(IssueData::MissingRequiredSubtasks { missing }),
    }]
}

#[cfg(test)]
mod tests;
