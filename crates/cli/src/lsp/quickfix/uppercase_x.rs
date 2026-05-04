use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Builds a quickfix for E007 (uppercase X in status box).
/// Replaces `[X]` with `[x]` by editing only the `X` character.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let line_idx = diagnostic.range.start.line as usize;
    let line_text = doc_text.lines().nth(line_idx)?;

    let pos = line_text.find("[X]")?;
    let x_col = (pos + 1) as u32;

    let text_edit = TextEdit {
        range: Range {
            start: Position {
                line: diagnostic.range.start.line,
                character: x_col,
            },
            end: Position {
                line: diagnostic.range.start.line,
                character: x_col + 1,
            },
        },
        new_text: "x".to_string(),
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![text_edit]);

    Some(CodeAction {
        title: "Replace [X] with [x]".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        command: None,
        disabled: None,
        data: None,
    })
}
