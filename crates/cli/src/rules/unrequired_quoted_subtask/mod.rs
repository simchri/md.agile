//! E011 — flags subtasks that use the `"quoted"` syntax but are not declared
//! as required by any property on their direct parent task or subtask.
//!
//! The `- [ ] "some text"` form is reserved exclusively for property-required
//! subtasks. Using it outside that context is a syntax error.

use crate::config::Config;
use crate::parser::{FileItem, Marker, SubtaskKind};
use crate::rules::{ErrorCode, Issue, IssueData, for_each_node};

pub fn unrequired_quoted_subtask(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        let required = collect_required(node.markers(), config);
        issues.extend(
            node.children()
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
                }),
        );
    });
    issues
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
