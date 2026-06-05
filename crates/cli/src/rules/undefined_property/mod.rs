use crate::config::Config;
use crate::parser::{FileItem, Marker, Subtask};
use crate::rules::{ErrorCode, Issue};

pub fn undefined_property(items: &[FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for item in items {
        if let FileItem::Task(task) = item {
            check_markers(
                &task.markers,
                &task.location,
                task.indent,
                config,
                &mut issues,
            );
            walk_subtasks(&task.children, config, &mut issues);
        }
    }
    issues
}

fn walk_subtasks(subtasks: &[Subtask], config: &Config, issues: &mut Vec<Issue>) {
    for sub in subtasks {
        check_markers(&sub.markers, &sub.location, sub.indent, config, issues);
        walk_subtasks(&sub.children, config, issues);
    }
}

fn check_markers(
    markers: &[Marker],
    location: &crate::parser::Location,
    indent: usize,
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    for marker in markers {
        if let Marker::Property(prop) = marker {
            if !config.properties.contains_key(&prop.name) {
                // Column is relative to the title; add indent + 6 for the task line prefix ("- [ ] ")
                let full_column = indent + 6 + prop.column;
                issues.push(Issue {
                    location: location.clone(),
                    code: ErrorCode::UndefinedProperty,
                    message: format!(
                        "Undefined property '#{}' — add '[Properties.{}]' to mdagile.toml",
                        prop.name, prop.name
                    ),
                    column: full_column,
                    help: None,
                    data: Some(crate::rules::IssueData::UndefinedProperty {
                        property_name: prop.name.clone(),
                    }),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
