use crate::rules::IssueData;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Builds a quickfix for E003 (wrong body indentation).
/// Extracts the expected_indent from diagnostic.data and replaces leading whitespace.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let data = diagnostic.data.as_ref()?;
    let issue_data: IssueData = serde_json::from_value(data.clone()).ok()?;

    let IssueData::WrongBodyIndent { expected_indent } = issue_data else {
        return None;
    };

    let line_idx = diagnostic.range.start.line as usize;
    let line_text = doc_text.lines().nth(line_idx)?;
    let current_indent = line_text.chars().take_while(|c| *c == ' ').count();

    let text_edit = TextEdit {
        range: Range {
            start: Position {
                line: diagnostic.range.start.line,
                character: 0,
            },
            end: Position {
                line: diagnostic.range.start.line,
                character: current_indent as u32,
            },
        },
        new_text: " ".repeat(expected_indent),
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![text_edit]);

    Some(CodeAction {
        title: format!("Fix indentation: use {} spaces", expected_indent),
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
