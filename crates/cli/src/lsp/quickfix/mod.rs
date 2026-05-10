use crate::rules::{ErrorCode, IssueData};
use std::collections::HashMap;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, Diagnostic, NumberOrString, TextEdit, Url, WorkspaceEdit,
};

mod invalid_box;
mod missing_space_after_box;
mod uppercase_x;
mod wrong_body_indent;
mod wrong_indentation;

/// Routes a diagnostic to the appropriate quickfix handler.
/// Dispatches based on error code; handlers don't re-check the code.
pub fn build_quickfix(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    match &diagnostic.code {
        Some(NumberOrString::String(s)) => match s.as_str() {
            code if code == ErrorCode::WrongIndentation.as_str() => {
                wrong_indentation::build(diagnostic, doc_text, uri)
            }
            code if code == ErrorCode::WrongBodyIndentation.as_str() => {
                wrong_body_indent::build(diagnostic, doc_text, uri)
            }
            code if code == ErrorCode::MissingSpaceAfterBox.as_str() => {
                missing_space_after_box::build(diagnostic, doc_text, uri)
            }
            code if code == ErrorCode::BoxStyleInvalid.as_str() => {
                invalid_box::build(diagnostic, doc_text, uri)
            }
            code if code == ErrorCode::UppercaseX.as_str() => {
                uppercase_x::build(diagnostic, doc_text, uri)
            }
            _ => None,
        },
        _ => None,
    }
}

/// Wraps a single [`TextEdit`] in the canonical [`CodeAction`] shape used by
/// every quickfix builder: kind = QUICKFIX, single-edit `WorkspaceEdit`,
/// `is_preferred = true`, no command/data.
fn make_quickfix(
    title: impl Into<String>,
    uri: &Url,
    diagnostic: &Diagnostic,
    edit: TextEdit,
) -> CodeAction {
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);
    CodeAction {
        title: title.into(),
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
    }
}

/// Deserializes the rule-specific payload attached to `diagnostic.data`.
///
/// Returns `None` if the diagnostic carries no data or the data does not
/// match any [`IssueData`] variant.
fn issue_data(diagnostic: &Diagnostic) -> Option<IssueData> {
    serde_json::from_value(diagnostic.data.as_ref()?.clone()).ok()
}

#[cfg(test)]
mod tests;
