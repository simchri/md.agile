use tower_lsp::lsp_types::*;

use crate::lsp::quickfix;

pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let line = 1;
    let x_col = 0;

    let edit = TextEdit {
        range: Range {
            start: Position {
                line,
                character: x_col,
            },
            end: Position {
                line,
                character: x_col + 1,
            },
        },
        new_text: "hello world".to_string(),
    };

    Some(super::make_quickfix(
        "Insert missing required subtasks",
        uri,
        diagnostic,
        edit,
    ))
}

#[cfg(test)]
#[path = "missing_subtasks_tests.rs"]
mod tests;
