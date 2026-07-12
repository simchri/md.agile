use crate::parser::{FileItem, ParsingIssue};
use crate::rules::{Issue, for_each_node};

pub fn invalid_box(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        if node.parsing_issues().contains(&ParsingIssue::InvalidBox) {
            issues.push(Issue {
                location: node.location().clone(),
                code: crate::rules::ErrorCode::BoxStyleInvalid,
                message: "Box style invalid".to_string(),
                // Position at the invalid character inside the status box
                // (or the closing `]` if the box is empty), relative to indent.
                column: node.indent() + 4,
                help: Some("Valid task boxes look like this: [ ] [x] [-]".to_string()),
                data: None,
            });
        }
    });
    issues
}

#[cfg(test)]
mod tests;
