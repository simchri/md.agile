use crate::parser::{FileItem, ParsingIssue};
use crate::rules::{Issue, for_each_node};

pub fn uppercase_x(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        if node.parsing_issues().contains(&ParsingIssue::UppercaseX) {
            issues.push(Issue {
                location: node.location().clone(),
                code: crate::rules::ErrorCode::UppercaseX,
                message: "Uppercase X in status box".to_string(),
                // Position at the `X` character inside the status box, relative to indent.
                column: node.indent() + 4,
                help: Some("Use lowercase: [x]".to_string()),
                data: None,
            });
        }
    });
    issues
}

#[cfg(test)]
mod tests;
