use crate::rules::IssueData;
use tower_lsp::lsp_types::*;

/// E002: re-indent a misaligned task line to the depth carried in the
/// diagnostic's [`IssueData::WrongIndent`] payload.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let IssueData::WrongIndent { expected_indent } = super::issue_data(diagnostic)? else {
        return None;
    };

    let line = diagnostic.range.start.line;
    let line_text = doc_text.lines().nth(line as usize)?;
    let current_indent = line_text.chars().take_while(|c| *c == ' ').count() as u32;

    let edit = TextEdit {
        range: Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: current_indent,
            },
        },
        new_text: " ".repeat(expected_indent),
    };

    Some(super::make_quickfix(
        format!("Fix indentation: use {expected_indent} spaces"),
        uri,
        diagnostic,
        edit,
    ))
}

#[cfg(test)]
#[path = "wrong_indentation_tests.rs"]
mod tests;
