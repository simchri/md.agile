use crate::config::Config;
use crate::parser::{Marker, TASK_LINE_PREFIX_LEN};
use crate::rules::{ErrorCode, Issue, for_each_node};

pub fn undefined_property(items: &[crate::parser::FileItem], config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |markers, location, indent| {
        for marker in markers {
            if let Marker::Property(prop) = marker {
                if !config.properties.contains_key(&prop.name) {
                    issues.push(Issue {
                        location: location.clone(),
                        code: ErrorCode::UndefinedProperty,
                        message: format!(
                            "Undefined property '#{}' — '[Properties.{}]' not in mdagile.toml",
                            prop.name, prop.name
                        ),
                        column: indent + TASK_LINE_PREFIX_LEN + prop.column,
                        help: None,
                        data: Some(crate::rules::IssueData::UndefinedProperty {
                            property_name: prop.name.clone(),
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
