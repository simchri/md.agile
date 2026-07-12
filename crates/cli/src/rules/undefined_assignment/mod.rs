use crate::config::Config;
use crate::parser::{Marker, TASK_LINE_PREFIX_LEN};
use crate::rules::{ErrorCode, Issue, for_each_node};

pub fn undefined_assignment(items: &[crate::parser::FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        for marker in node.markers() {
            if let Marker::Assignment(assignment) = marker {
                let declared = config.users.contains_key(&assignment.name)
                    || config.groups.contains_key(&assignment.name);
                if !declared {
                    issues.push(Issue {
                        location: node.location().clone(),
                        code: ErrorCode::UndefinedAssignment,
                        message: format!(
                            "Undefined assignment '@{}' — '[Users.{}]' or '[Groups.{}]' not in mdagile.toml",
                            assignment.name, assignment.name, assignment.name
                        ),
                        column: node.indent() + TASK_LINE_PREFIX_LEN + assignment.column,
                        help: None,
                        data: Some(crate::rules::IssueData::UndefinedAssignment {
                            assignment_name: assignment.name.clone(),
                        }),
                    });
                }
            }
        }
    });
    issues
}

#[cfg(test)]
mod tests;
