use crate::config::Config;
use crate::rules::IssueData;
use tower_lsp::lsp_types::*;

/// E024: insert each missing required form field as a new body line after
/// the last existing line of the flagged task/subtask's subtree (same
/// "append after the whole subtree" placement as [`super::missing_subtasks`]
/// and [`super::missing_neighbor_tasks`]).
///
/// Each inserted line is indented at `node_indent + 2` and formatted as
/// `__Label:__ <value>`, where `<value>` is that field's configured
/// `default` (looked up from `mdagile.toml` by matching label), falling back
/// to the generic unfilled placeholder `___` when the field has no
/// configured default or its config can't be found at all (e.g. `mdagile.toml`
/// failed to load).
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let IssueData::MissingRequiredField { missing } = super::issue_data(diagnostic)? else {
        return None;
    };

    if missing.is_empty() {
        return None;
    }

    // Best-effort: a missing/unparsable config just means every field falls
    // back to the generic placeholder, same as the parser default.
    let config = super::read_toml(uri).and_then(|(_, text)| Config::from_str(&text).ok());

    let node_line = diagnostic.range.start.line;
    let lines: Vec<&str> = doc_text.lines().collect();
    let node_line_text = lines.get(node_line as usize)?;
    let node_indent = node_line_text.chars().take_while(|c| *c == ' ').count();
    let field_indent = node_indent + 2;

    let last_line = super::missing_subtasks::find_last_subtree_line(&lines, node_line, node_indent);
    let last_line_text = lines.get(last_line as usize)?;
    let last_line_len = last_line_text.len() as u32;

    let insert_text: String = missing
        .iter()
        .map(|label| {
            let value = config
                .as_ref()
                .and_then(|c| default_for_label(c, label))
                .unwrap_or_else(|| "___".to_string());
            format!("\n{}__{}:__ {}", " ".repeat(field_indent), label, value)
        })
        .collect::<Vec<_>>()
        .join("");

    let edit = TextEdit {
        range: Range {
            start: Position {
                line: last_line,
                character: last_line_len,
            },
            end: Position {
                line: last_line,
                character: last_line_len,
            },
        },
        new_text: insert_text,
    };

    Some(super::make_quickfix(
        "Insert missing required field(s)",
        uri,
        diagnostic,
        edit,
    ))
}

/// Finds the configured `default` for the `[Field.X]` whose `label` matches
/// `label` exactly (the same matching key the E024 check itself uses).
/// Returns `None` both when no field has that label and when the matching
/// field has no configured default (plain `___` placeholder).
fn default_for_label(config: &Config, label: &str) -> Option<String> {
    config
        .fields
        .values()
        .find(|f| f.label == label)
        .and_then(|f| f.default.clone())
}

#[cfg(test)]
#[path = "missing_required_field_tests.rs"]
mod tests;
