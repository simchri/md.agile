use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Builds a quickfix for E006 (invalid box style).
/// Replaces the entire `[…]` (e.g. `[]`, `[o]`, `[xx]`) with `[ ]`.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let line_idx = diagnostic.range.start.line as usize;
    let line_text = doc_text.lines().nth(line_idx)?;

    let open = line_text.find('[')?;
    let close_rel = line_text[open + 1..].find(']')?;
    let close = open + 1 + close_rel;

    let text_edit = TextEdit {
        range: Range {
            start: Position {
                line: diagnostic.range.start.line,
                character: open as u32,
            },
            end: Position {
                line: diagnostic.range.start.line,
                character: (close + 1) as u32,
            },
        },
        new_text: "[ ]".to_string(),
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![text_edit]);

    Some(CodeAction {
        title: "Replace with empty box: [ ]".to_string(),
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
