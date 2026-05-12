use crate::rules::{ErrorCode, IssueData};
use std::collections::HashMap;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, Diagnostic, NumberOrString, TextEdit, Url, WorkspaceEdit,
};

mod invalid_box;
mod missing_space_after_box;
mod undefined_property;
mod uppercase_x;
mod wrong_body_indent;
mod wrong_indentation;

/// Function shape every quickfix builder satisfies.
type Builder = fn(&Diagnostic, &str, &Url) -> Option<CodeAction>;

/// Single source of truth for which [`ErrorCode`]s have a quickfix and which
/// builder produces it. Used by both [`build_quickfix`] (LSP dispatch) and
/// [`has_quickfix`] (CLI's "(fix avail.)" hint).
const REGISTRY: &[(ErrorCode, Builder)] = &[
    (ErrorCode::WrongIndentation, wrong_indentation::build),
    (ErrorCode::WrongBodyIndentation, wrong_body_indent::build),
    (
        ErrorCode::MissingSpaceAfterBox,
        missing_space_after_box::build,
    ),
    (ErrorCode::BoxStyleInvalid, invalid_box::build),
    (ErrorCode::UppercaseX, uppercase_x::build),
    (ErrorCode::UndefinedProperty, undefined_property::build),
    // E001 OrphanedSubtask, E004 IncompleteParent:
    // no quickfix (user has to make a structural decision the linter can't).
];

/// Routes a diagnostic to the appropriate quickfix handler via [`REGISTRY`].
pub fn build_quickfix(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let Some(NumberOrString::String(s)) = &diagnostic.code else {
        return None;
    };
    let code: ErrorCode = s.parse().ok()?;
    let builder = REGISTRY.iter().find(|(c, _)| *c == code).map(|(_, b)| *b)?;
    builder(diagnostic, doc_text, uri)
}

/// Returns true if `code` has a registered quickfix builder.
pub fn has_quickfix(code: ErrorCode) -> bool {
    REGISTRY.iter().any(|(c, _)| *c == code)
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
