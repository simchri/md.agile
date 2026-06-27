use tower_lsp::lsp_types::*;

/// E009 quickfix builder.
///
/// May return up to three actions:
/// 1. When the typed name is within [`super::MAX_EDIT_DISTANCE`] edits of a
///    known user or group: correct the spelling in the `.agile.md` document
///    (preferred; listed first).
/// 2. Add the name as `[Users.X]` to `mdagile.toml`.
/// 3. Add the name as `[Groups.X]` to `mdagile.toml`.
///
/// Actions 2 and 3 are always offered (the intent -- user vs. group -- is
/// unknown at the point of the error) and are deprioritised when a spelling
/// correction is available.
pub fn build(diagnostic: &Diagnostic, _doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    let issue_data = match super::issue_data(diagnostic) {
        Some(d) => d,
        None => return vec![],
    };
    let assignment_name = match issue_data {
        crate::rules::IssueData::UndefinedAssignment { assignment_name } => assignment_name,
        _ => return vec![],
    };

    let Some((toml_path, toml_content)) = super::read_toml(uri) else {
        return vec![];
    };

    let corrections = super::build_spelling_corrections(
        diagnostic,
        uri,
        &assignment_name,
        &toml_content,
        &["Users", "Groups"],
        '@',
    );
    let have_corrections = !corrections.is_empty();
    let mut actions: Vec<CodeAction> = corrections;

    for section in ["Users", "Groups"] {
        if let Some(mut add) = super::build_add_to_toml(
            diagnostic,
            &assignment_name,
            &toml_path,
            &toml_content,
            section,
        ) {
            if have_corrections {
                add.is_preferred = Some(false);
            }
            actions.push(add);
        }
    }

    actions
}

#[cfg(test)]
#[path = "undefined_assignment_tests.rs"]
mod tests;
