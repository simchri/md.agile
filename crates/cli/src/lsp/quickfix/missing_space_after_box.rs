use crate::rules::IssueData;
use tower_lsp::lsp_types::*;

/// E005: insert a space after the first `]` on the line.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let IssueData::MissingSpaceAfterBox = super::issue_data(diagnostic)? else {
        return None;
    };

    let line = diagnostic.range.start.line;
    let line_text = doc_text.lines().nth(line as usize)?;
    let after_bracket = (line_text.find(']')? + 1) as u32;

    let edit = TextEdit {
        range: Range {
            start: Position {
                line,
                character: after_bracket,
            },
            end: Position {
                line,
                character: after_bracket,
            },
        },
        new_text: " ".to_string(),
    };

    Some(super::make_quickfix(
        "Add space after status box",
        uri,
        diagnostic,
        edit,
    ))
}
