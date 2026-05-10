use tower_lsp::lsp_types::*;

/// E006: replace an invalid `[…]` (e.g. `[]`, `[o]`, `[xx]`) with `[ ]`.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let line = diagnostic.range.start.line;
    let line_text = doc_text.lines().nth(line as usize)?;

    let open = line_text.find('[')?;
    let close = open + 1 + line_text[open + 1..].find(']')?;

    let edit = TextEdit {
        range: Range {
            start: Position {
                line,
                character: open as u32,
            },
            end: Position {
                line,
                character: (close + 1) as u32,
            },
        },
        new_text: "[ ]".to_string(),
    };

    Some(super::make_quickfix(
        "Replace with empty box: [ ]",
        uri,
        diagnostic,
        edit,
    ))
}
