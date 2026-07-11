//! E010 — flags tasks/subtasks that are missing required subtasks mandated by a property.
//! E012 — flags a required subtask that was cancelled without the property allowing it.

use crate::config::Config;
use crate::parser::Location;
use crate::parser::{FileItem, Marker, Status, Subtask, SubtaskKind};
use crate::rules::{ErrorCode, Issue, IssueData};
use std::collections::HashMap;

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
    // merging across multiple properties and deduplicating. `allow_cancel` records,
    // per required subtask string, whether *any* property on this node permits it to
    // be satisfied by cancellation rather than completion.
    let mut required: Vec<String> = Vec::new();
    let mut allow_cancel: HashMap<String, bool> = HashMap::new();
    for marker in markers {
        if let Marker::Property(prop) = marker {
            if let Some(prop_config) = config.properties.get(&prop.name) {
                for (i, s) in prop_config.subtasks.iter().enumerate() {
                    if !required.contains(s) {
                        required.push(s.clone());
                    }
                    let allowed = prop_config
                        .subtasks_allow_cancel
                        .get(i)
                        .copied()
                        .unwrap_or(false);
                    let entry = allow_cancel.entry(s.clone()).or_insert(false);
                    *entry = *entry || allowed;
                }
            }
        }
    }

    if required.is_empty() {
        return vec![];
    }

    // Collect all PropertyRequired direct children, keyed by their raw_title.
    let present: Vec<&Subtask> = children
        .iter()
        .filter(|c| c.kind == SubtaskKind::PropertyRequired)
        .collect();

    let mut issues = Vec::new();

    // A required subtask that's cancelled without permission is reported as E012,
    // in addition to (still) counting as "present" for the E010 missing-check below.
    for child in &present {
        let Some(title) = child.raw_title.as_deref() else {
            continue;
        };
        if child.status == Status::Cancelled
            && required.iter().any(|r| r == title)
            && !allow_cancel.get(title).copied().unwrap_or(false)
        {
            issues.push(Issue {
                location: child.location.clone(),
                code: ErrorCode::CancelledRequiredSubtaskNotAllowed,
                message: format!(
                    "Required subtask \"{title}\" was cancelled, but its property doesn't allow cancellation"
                ),
                column: 1,
                help: Some(
                    "Complete this subtask, or add it to `subtasks_allow_cancel` in mdagile.toml to permit cancelling it.".to_string(),
                ),
                data: Some(IssueData::CancelledRequiredSubtaskNotAllowed {
                    title: title.to_string(),
                }),
            });
        }
    }

    let present_titles: Vec<&str> = present
        .iter()
        .filter_map(|c| c.raw_title.as_deref())
        .collect();

    let missing: Vec<String> = required
        .iter()
        .filter(|r| !present_titles.contains(&r.as_str()))
        .cloned()
        .collect();

    if !missing.is_empty() {
        let missing_quoted: Vec<String> = missing.iter().map(|s| format!("\"{s}\"")).collect();
        issues.push(Issue {
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
        });
    }

    issues
}

#[cfg(test)]
mod tests;
