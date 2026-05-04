use crate::rules::IssueData;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Builds a quickfix for E005 (missing space after status box).
/// Finds the first `]` in the line and inserts a space after it.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let data = diagnostic.data.as_ref()?;
    let issue_data: IssueData = serde_json::from_value(data.clone()).ok()?;

    let IssueData::MissingSpaceAfterBox = issue_data else {
        return None;
    };

    let line_idx = diagnostic.range.start.line as usize;
    let line_text = doc_text.lines().nth(line_idx)?;

    // Find the first `]` in the line and insert a space after it
    if let Some(bracket_pos) = line_text.find(']') {
        let text_edit = TextEdit {
            range: Range {
                start: Position {
                    line: diagnostic.range.start.line,
                    character: (bracket_pos + 1) as u32,
                },
                end: Position {
                    line: diagnostic.range.start.line,
                    character: (bracket_pos + 1) as u32,
                },
            },
            new_text: " ".to_string(),
        };

        let mut changes = HashMap::new();
        changes.insert(uri.clone(), vec![text_edit]);

        Some(CodeAction {
            title: "Add space after status box".to_string(),
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
    } else {
        None
    }
}
