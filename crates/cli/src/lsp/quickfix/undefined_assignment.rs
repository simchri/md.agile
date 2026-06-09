use tower_lsp::lsp_types::*;

/// E009 quickfix builder — may return up to three actions:
/// 1. When the typed name is within [`super::MAX_EDIT_DISTANCE`] edits of a
///    known user or group: correct the spelling in the `.agile.md` document
///    (preferred; listed first).
/// 2. Add the name as `[Users.X]` to `mdagile.toml`.
/// 3. Add the name as `[Groups.X]` to `mdagile.toml`.
///
/// Actions 2 and 3 are always offered (the intent — user vs. group — is
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

    let corrections = build_spelling_corrections(diagnostic, uri, &assignment_name);
    let have_corrections = !corrections.is_empty();

    let mut actions: Vec<CodeAction> = corrections;

    for section in ["Users", "Groups"] {
        if let Some(mut add) = build_add_to_toml(diagnostic, &assignment_name, uri, section) {
            if have_corrections {
                add.is_preferred = Some(false);
            }
            actions.push(add);
        }
    }

    actions
}

fn build_add_to_toml(
    diagnostic: &Diagnostic,
    assignment_name: &str,
    uri: &Url,
    section: &str,
) -> Option<CodeAction> {
    let toml_path = super::find_toml_path(uri)?;
    let current_content = std::fs::read_to_string(&toml_path).unwrap_or_default();

    let new_content = if current_content.is_empty() {
        format!("[{}.{}]\n", section, assignment_name)
    } else {
        let mut content = current_content;
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("[{}.{}]\n", section, assignment_name));
        content
    };

    let toml_uri = Url::from_file_path(&toml_path).ok()?;
    let edit = TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: u32::MAX,
                character: u32::MAX,
            },
        },
        new_text: new_content,
    };

    Some(super::make_quickfix(
        format!("Add '[{}.{}]' to mdagile.toml", section, assignment_name),
        &toml_uri,
        diagnostic,
        edit,
    ))
}

fn build_spelling_corrections(
    diagnostic: &Diagnostic,
    uri: &Url,
    typed_name: &str,
) -> Vec<CodeAction> {
    let toml_path = match super::find_toml_path(uri) {
        Some(p) => p,
        None => return vec![],
    };
    let toml_content = match std::fs::read_to_string(&toml_path) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    // Collect names from both [Users.NAME] and [Groups.NAME] sections.
    let existing: Vec<String> = toml_content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let inner = line
                .strip_prefix("[Users.")
                .or_else(|| line.strip_prefix("[Groups."))?;
            let name = inner.strip_suffix(']')?;
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect();

    // The '@' character position is stored in diagnostic.range.end.character
    // (set by issue_to_diagnostic as the 0-based column of the marker start).
    let at_col = diagnostic.range.end.character;
    let line = diagnostic.range.start.line;
    let token_len = (1 + typed_name.len()) as u32; // '@' + name

    existing
        .into_iter()
        .filter(|known| super::levenshtein(typed_name, known) <= super::MAX_EDIT_DISTANCE)
        .map(|correct_name| {
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line,
                        character: at_col,
                    },
                    end: Position {
                        line,
                        character: at_col + token_len,
                    },
                },
                new_text: format!("@{}", correct_name),
            };
            super::make_quickfix(
                format!("Fix typo: replace '@{typed_name}' with '@{correct_name}'"),
                uri,
                diagnostic,
                edit,
            )
        })
        .collect()
}
