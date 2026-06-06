use tower_lsp::lsp_types::*;

const MAX_EDIT_DISTANCE: usize = 2;

/// E008 quickfix builder — may return up to two actions:
/// 1. Always: add the undefined property to `mdagile.toml`.
/// 2. When the typed name is within [`MAX_EDIT_DISTANCE`] edits of an
///    existing property: correct the spelling in the `.agile.md` document.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    let issue_data = match super::issue_data(diagnostic) {
        Some(d) => d,
        None => return vec![],
    };
    let property_name = match issue_data {
        crate::rules::IssueData::UndefinedProperty { property_name } => property_name,
        _ => return vec![],
    };

    let mut actions = Vec::new();

    // --- Action 1: add the property to mdagile.toml ---
    if let Some(add_action) = build_add_to_toml(diagnostic, &property_name, uri) {
        actions.push(add_action);
    }

    // --- Action 2: suggest spelling corrections (if close match exists) ---
    let corrections = build_spelling_corrections(diagnostic, doc_text, uri, &property_name);
    actions.extend(corrections);

    actions
}

/// Builds the "Add '[Properties.X]' to mdagile.toml" action.
fn build_add_to_toml(
    diagnostic: &Diagnostic,
    property_name: &str,
    uri: &Url,
) -> Option<CodeAction> {
    let file_path = uri.to_file_path().ok()?;

    // Find mdagile.toml by walking up the directory tree
    let mut dir = file_path.parent()?;
    let toml_path = loop {
        let plain = dir.join("mdagile.toml");
        let dot = dir.join(".mdagile.toml");
        if plain.exists() {
            break plain;
        }
        if dot.exists() {
            break dot;
        }
        dir = dir.parent()?;
    };

    // Read the current toml file
    let current_content = std::fs::read_to_string(&toml_path).unwrap_or_default();

    // Build the new content: append the new property section
    let new_content = if current_content.is_empty() {
        format!("[Properties.{}]\n", property_name)
    } else {
        let mut content = current_content;
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("[Properties.{}]\n", property_name));
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
        format!("Add '[Properties.{}]' to mdagile.toml", property_name),
        &toml_uri,
        diagnostic,
        edit,
    ))
}

/// Returns spelling-correction actions for each known property that is within
/// [`MAX_EDIT_DISTANCE`] edits of `typed_name`.
fn build_spelling_corrections(
    diagnostic: &Diagnostic,
    _doc_text: &str,
    uri: &Url,
    typed_name: &str,
) -> Vec<CodeAction> {
    let file_path = match uri.to_file_path() {
        Ok(p) => p,
        Err(_) => return vec![],
    };

    // Find mdagile.toml
    let toml_path = {
        let mut dir = match file_path.parent() {
            Some(d) => d,
            None => return vec![],
        };
        loop {
            let plain = dir.join("mdagile.toml");
            let dot = dir.join(".mdagile.toml");
            if plain.exists() {
                break plain;
            }
            if dot.exists() {
                break dot;
            }
            match dir.parent() {
                Some(p) => dir = p,
                None => return vec![],
            }
        }
    };

    let toml_content = match std::fs::read_to_string(&toml_path) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    // Extract existing property names from [Properties.NAME] sections
    let existing: Vec<String> = toml_content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let inner = line.strip_prefix("[Properties.")?;
            let name = inner.strip_suffix(']')?;
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect();

    // The '#' character position in the document is stored in
    // diagnostic.range.end.character (set by issue_to_diagnostic as dash_col).
    let hash_col = diagnostic.range.end.character;
    let line = diagnostic.range.start.line;
    let token_len = (1 + typed_name.len()) as u32; // '#' + name

    existing
        .into_iter()
        .filter(|known| levenshtein(typed_name, known) <= MAX_EDIT_DISTANCE)
        .map(|correct_name| {
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line,
                        character: hash_col,
                    },
                    end: Position {
                        line,
                        character: hash_col + token_len,
                    },
                },
                new_text: format!("#{}", correct_name),
            };
            super::make_quickfix(
                format!("Fix typo: replace '#{typed_name}' with '#{correct_name}'"),
                uri,
                diagnostic,
                edit,
            )
        })
        .collect()
}

/// Computes the Levenshtein edit distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();

    // Early-exit: length difference alone exceeds threshold
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

#[cfg(test)]
mod levenshtein_tests {
    use super::levenshtein;

    #[test]
    fn identical_strings_have_distance_zero() {
        assert_eq!(levenshtein("feature", "feature"), 0);
    }

    #[test]
    fn single_deletion() {
        assert_eq!(levenshtein("feture", "feature"), 1);
    }

    #[test]
    fn single_substitution() {
        assert_eq!(levenshtein("feaxure", "feature"), 1);
    }

    #[test]
    fn single_insertion() {
        assert_eq!(levenshtein("features", "feature"), 1);
    }

    #[test]
    fn completely_different() {
        // "xyz" vs "completelydifferent" — length diff alone exceeds threshold
        assert!(levenshtein("xyz", "completelydifferent") > 2);
    }
}
