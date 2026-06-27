use tower_lsp::lsp_types::*;

/// E007: replace `[X]` with `[x]` by editing only the `X` character.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let line = diagnostic.range.start.line;
    let line_text = doc_text.lines().nth(line as usize)?;
    let x_col = (line_text.find("[X]")? + 1) as u32;

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
        new_text: "x".to_string(),
    };

    Some(super::make_quickfix(
        "Replace [X] with [x]",
        uri,
        diagnostic,
        edit,
    ))
}

#[cfg(test)]
#[path = "uppercase_x_tests.rs"]
mod tests;
