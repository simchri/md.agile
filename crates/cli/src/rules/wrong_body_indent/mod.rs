//! E003 — flags task/subtask description lines with misaligned indentation.

use crate::parser::{FileItem, Location};
use crate::rules::{Issue, for_each_node};

/// Flags task and subtask body lines that don't match the expected indentation.
///
/// For a task/subtask at indentation level `indent`, body lines should be
/// indented to exactly `indent + 2` spaces. Lines that deviate from this
/// are flagged as wrong body indent.
pub fn wrong_body_indent(items: &[FileItem]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_node(items, |node| {
        let expected_indent = node.indent() + 2;
        for (idx, line) in node.body().iter().enumerate() {
            let actual_indent = line.chars().take_while(|c| *c == ' ').count();
            if actual_indent != expected_indent && !line.trim().is_empty() {
                // Body lines come after the task/subtask title line.
                let body_line_number = node.location().line + 1 + idx;
                issues.push(Issue {
                    location: Location {
                        path: node.location().path.clone(),
                        line: body_line_number,
                    },
                    code: crate::rules::ErrorCode::WrongBodyIndentation,
                    message: "Wrong body indentation".to_string(),
                    column: actual_indent + 1,
                    help: Some(format!(
                        "Body lines should be indented to {} space{}, but this line has {}.",
                        expected_indent,
                        if expected_indent == 1 { "" } else { "s" },
                        actual_indent
                    )),
                    data: Some(crate::rules::IssueData::WrongBodyIndent { expected_indent }),
                });
            }
        }
    });
    issues
}

#[cfg(test)]
mod tests;
