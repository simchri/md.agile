use crate::rules::ErrorCode;
use tower_lsp::lsp_types::{CodeAction, Diagnostic, NumberOrString, Url};

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

#[cfg(test)]
mod tests;
