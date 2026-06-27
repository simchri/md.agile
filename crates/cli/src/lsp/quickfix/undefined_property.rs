use tower_lsp::lsp_types::*;

/// E008 quickfix builder.
///
/// May return up to two actions:
/// 1. When the typed name is within [`super::MAX_EDIT_DISTANCE`] edits of an
///    existing property: correct the spelling in the `.agile.md` document
///    (preferred; listed first).
/// 2. Add the undefined property to `mdagile.toml` (deprioritised when a
///    spelling correction is available).
pub fn build(diagnostic: &Diagnostic, _doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    let issue_data = match super::issue_data(diagnostic) {
        Some(d) => d,
        None => return vec![],
    };
    let property_name = match issue_data {
        crate::rules::IssueData::UndefinedProperty { property_name } => property_name,
        _ => return vec![],
    };

    let Some((toml_path, toml_content)) = super::read_toml(uri) else {
        return vec![];
    };

    let corrections = super::build_spelling_corrections(
        diagnostic,
        uri,
        &property_name,
        &toml_content,
        &["Properties"],
        '#',
    );
    let have_corrections = !corrections.is_empty();
    let mut actions: Vec<CodeAction> = corrections;

    if let Some(mut add) = super::build_add_to_toml(
        diagnostic,
        &property_name,
        &toml_path,
        &toml_content,
        "Properties",
    ) {
        if have_corrections {
            add.is_preferred = Some(false);
        }
        actions.push(add);
    }

    actions
}

#[cfg(test)]
#[path = "undefined_property_tests.rs"]
mod tests;
