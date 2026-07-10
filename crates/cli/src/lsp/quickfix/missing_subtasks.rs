use tower_lsp::lsp_types::*;

use crate::lsp::quickfix;

pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let line = 1;
    let x_col = 0;

    let result: Result<serde_json::Value, _> = serde_json::from_value(diagnostic.data.clone().unwrap_or_default());

    let strings: Vec<String> = match result {
        Ok(payload) => {
            serde_json::from_value(payload["missing"].clone()).unwrap_or_default()
        }
        Err(_) => {
            log::warn!("Failed to unwrap diagnostic data, expected a Vec<String> of missing subtasks, but got: {:?}", diagnostic.data);
            vec![]
        }
    };

    let mut missing_task = "dummy".into();

    if strings.len() > 0 {
        missing_task = strings.join("\n- [ ] ");
    }

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
        new_text: missing_task.to_string(),
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
