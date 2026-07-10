use crate::rules::{ErrorCode, IssueData};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, Diagnostic, NumberOrString, Position, Range, TextEdit, Url,
    WorkspaceEdit,
};

mod invalid_box;
mod missing_space_after_box;
mod undefined_assignment;
mod undefined_property;
mod uppercase_x;
mod wrong_body_indent;
mod wrong_indentation;
mod missing_subtasks;

/// Function shape every quickfix builder satisfies.
/// Returns all applicable code actions for the diagnostic (zero or more).
type Builder = fn(&Diagnostic, &str, &Url) -> Vec<CodeAction>;

/// Single source of truth for which [`ErrorCode`]s have a quickfix and which
/// builder produces it. Used by both [`build_quickfixes`] (LSP dispatch) and
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
    (ErrorCode::UndefinedAssignment, undefined_assignment::build),
    (ErrorCode::MissingRequiredSubtasks, missing_subtasks::build),

    // E001 OrphanedSubtask, E004 IncompleteParent:
    // no quickfix (user has to make a structural decision the linter can't).
];

/// Routes a diagnostic to all applicable quickfix handlers via [`REGISTRY`].
///
/// Returns every [`CodeAction`] produced by the matching builder — usually one,
/// but E008 (undefined property) can return multiple when a close-match
/// spelling correction is also available.
pub fn build_quickfixes(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    let Some(NumberOrString::String(s)) = &diagnostic.code else {
        return vec![];
    };
    let Ok(code) = s.parse::<ErrorCode>() else {
        return vec![];
    };
    match REGISTRY.iter().find(|(c, _)| *c == code).map(|(_, b)| *b) {
        Some(builder) => builder(diagnostic, doc_text, uri),
        None => vec![],
    }
}

/// Convenience wrapper — returns the first (preferred) action, if any.
///
/// Use [`build_quickfixes`] when you need all available actions.
pub fn build_quickfix(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    build_quickfixes(diagnostic, doc_text, uri)
        .into_iter()
        .next()
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

/// Walk up from the directory of `uri` to find the nearest `mdagile.toml`
/// or `.mdagile.toml`. Returns `None` if neither is found.
fn find_toml_path(uri: &Url) -> Option<PathBuf> {
    let file_path = uri.to_file_path().ok()?;
    let mut dir = file_path.parent()?;
    loop {
        let plain = dir.join("mdagile.toml");
        let dot = dir.join(".mdagile.toml");
        if plain.exists() {
            return Some(plain);
        }
        if dot.exists() {
            return Some(dot);
        }
        dir = dir.parent()?;
    }
}

/// Finds and reads the nearest `mdagile.toml`. Returns the resolved path and
/// file contents, or `None` if no toml file is found or it cannot be read.
///
/// Callers that need both spelling corrections and add-to-toml actions should
/// call this once and pass the results to [`build_spelling_corrections`] and
/// [`build_add_to_toml`], avoiding multiple I/O round-trips.
pub(super) fn read_toml(uri: &Url) -> Option<(PathBuf, String)> {
    let path = find_toml_path(uri)?;
    let content = std::fs::read_to_string(&path).ok()?;
    Some((path, content))
}

pub(super) const MAX_EDIT_DISTANCE: usize = 2;

/// Levenshtein edit distance between `a` and `b`.
/// Returns `MAX_EDIT_DISTANCE + 1` early when the length difference already
/// exceeds the threshold (avoids allocating the full DP table).
pub(super) fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();

    if m.abs_diff(n) > MAX_EDIT_DISTANCE {
        return MAX_EDIT_DISTANCE + 1;
    }

    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j - 1].min(dp[i - 1][j]).min(dp[i][j - 1])
            };
        }
    }
    dp[m][n]
}

/// Collects names declared under any of `sections` in `toml_content`.
///
/// E.g. `sections = &["Properties"]` matches `[Properties.feature]` → `"feature"`.
fn extract_toml_names(toml_content: &str, sections: &[&str]) -> Vec<String> {
    toml_content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            sections.iter().find_map(|section| {
                let prefix = format!("[{}.", section);
                let inner = line.strip_prefix(&prefix)?;
                let name = inner.strip_suffix(']')?;
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                }
            })
        })
        .collect()
}

/// Builds "Fix typo" quickfixes for `typed_name` against all names declared
/// under any of `sections` in `toml_content`.
///
/// `sigil` is `'#'` for properties or `'@'` for assignments. The column of the
/// sigil in the document is read from `diagnostic.range.end.character`.
pub(super) fn build_spelling_corrections(
    diagnostic: &Diagnostic,
    uri: &Url,
    typed_name: &str,
    toml_content: &str,
    sections: &[&str],
    sigil: char,
) -> Vec<CodeAction> {
    let existing = extract_toml_names(toml_content, sections);
    let marker_col = diagnostic.range.end.character;
    let line = diagnostic.range.start.line;
    let token_len = (1 + typed_name.len()) as u32; // sigil + name

    existing
        .into_iter()
        .filter(|known| levenshtein(typed_name, known) <= MAX_EDIT_DISTANCE)
        .map(|correct_name| {
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line,
                        character: marker_col,
                    },
                    end: Position {
                        line,
                        character: marker_col + token_len,
                    },
                },
                new_text: format!("{}{}", sigil, correct_name),
            };
            make_quickfix(
                format!("Fix typo: replace '{sigil}{typed_name}' with '{sigil}{correct_name}'"),
                uri,
                diagnostic,
                edit,
            )
        })
        .collect()
}

/// Builds an "Add '[section.name]' to mdagile.toml" quickfix by appending a
/// new TOML section header to the file, preserving existing content.
pub(super) fn build_add_to_toml(
    diagnostic: &Diagnostic,
    name: &str,
    toml_path: &Path,
    toml_content: &str,
    section: &str,
) -> Option<CodeAction> {
    let new_content = if toml_content.is_empty() {
        format!("[{}.{}]\n", section, name)
    } else {
        let mut content = toml_content.to_string();
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("[{}.{}]\n", section, name));
        content
    };

    let toml_uri = Url::from_file_path(toml_path).ok()?;
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

    Some(make_quickfix(
        format!("Add '[{}.{}]' to mdagile.toml", section, name),
        &toml_uri,
        diagnostic,
        edit,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_quickfix_for_each_code() {
        use crate::rules::ErrorCode::*;
        assert!(!has_quickfix(OrphanedSubtask));
        assert!(has_quickfix(WrongIndentation));
        assert!(has_quickfix(WrongBodyIndentation));
        assert!(!has_quickfix(IncompleteParent));
        assert!(has_quickfix(MissingSpaceAfterBox));
        assert!(has_quickfix(BoxStyleInvalid));
        assert!(has_quickfix(UppercaseX));
        assert!(has_quickfix(UndefinedProperty));
        assert!(has_quickfix(UndefinedAssignment));
    }

    // --- levenshtein unit tests ---

    #[test]
    fn levenshtein_identical_strings() {
        assert_eq!(levenshtein("feature", "feature"), 0);
    }

    #[test]
    fn levenshtein_single_deletion() {
        assert_eq!(levenshtein("feture", "feature"), 1);
    }

    #[test]
    fn levenshtein_single_substitution() {
        assert_eq!(levenshtein("feaxure", "feature"), 1);
    }

    #[test]
    fn levenshtein_single_insertion() {
        assert_eq!(levenshtein("features", "feature"), 1);
    }

    #[test]
    fn levenshtein_far_apart_strings() {
        // Length diff alone exceeds MAX_EDIT_DISTANCE -> capped at MAX+1
        assert!(levenshtein("xyz", "completelydifferent") > MAX_EDIT_DISTANCE);
    }
}
