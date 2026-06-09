use crate::config::Config;
use crate::parser::{FileItem, Marker, Subtask};
use crate::rules::{ErrorCode, Issue};

pub fn undefined_assignment(items: &[FileItem], config: &Config) -> Vec<Issue> {
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
        if let Marker::Assignment(assignment) = marker {
            let declared = config.users.contains_key(&assignment.name)
                || config.groups.contains_key(&assignment.name);
            if !declared {
                let full_column = indent + 6 + assignment.column;
                issues.push(Issue {
                    location: location.clone(),
                    code: ErrorCode::UndefinedAssignment,
                    message: format!(
                        "Undefined assignment '@{}' — '[Users.{}]' or '[Groups.{}]' not in mdagile.toml",
                        assignment.name, assignment.name, assignment.name
                    ),
                    column: full_column,
                    help: None,
                    data: Some(crate::rules::IssueData::UndefinedAssignment {
                        assignment_name: assignment.name.clone(),
                    }),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
