use tower_lsp::lsp_types::{CodeAction, Diagnostic, NumberOrString, Url};

mod wrong_indentation;
mod wrong_body_indent;
mod missing_space_after_box;

/// Routes a diagnostic to the appropriate quickfix handler.
/// Dispatches based on error code string; handlers don't re-check the code.
pub fn build_quickfix(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    match &diagnostic.code {
        Some(NumberOrString::String(s)) => match s.as_str() {
            "E002" => wrong_indentation::build(diagnostic, doc_text, uri),
            "E003" => wrong_body_indent::build(diagnostic, doc_text, uri),
            "E005" => missing_space_after_box::build(diagnostic, doc_text, uri),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests;
