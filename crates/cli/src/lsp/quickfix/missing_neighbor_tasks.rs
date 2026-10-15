use crate::rules::IssueData;
use tower_lsp::lsp_types::*;

/// E019: insert each missing neighbor task as a new sibling line, placed
/// immediately after the subtree of the property-carrying task/subtask (same
/// indentation level, quoted to match the PropertyRequired convention
/// required subtasks already use — see `crate::rules::neighbor_tasks`).
/// Inserted right after the flagged node's own subtree, which may land it
/// before a later, unrelated sibling — harmless, since neighbor tasks aren't
/// ordered relative to other siblings.
///
/// Produces no action when the flagged node is a top-level task (indentation
/// 0): a top-level task has no quoted-sibling concept at all, so the
/// requirement can never be satisfied there and there's no edit to offer
/// (see the `neighbor_tasks` module docs).
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let IssueData::MissingNeighborTask { missing } = super::issue_data(diagnostic)? else {
        return None;
    };

    if missing.is_empty() {
        return None;
    }

    let node_line = diagnostic.range.start.line;
    let lines: Vec<&str> = doc_text.lines().collect();
    let node_line_text = lines.get(node_line as usize)?;
    let node_indent = node_line_text.chars().take_while(|c| *c == ' ').count();

    if node_indent == 0 {
        return None;
    }

    let last_line = super::missing_subtasks::find_last_subtree_line(&lines, node_line, node_indent);
    let last_line_text = lines.get(last_line as usize)?;
    let last_line_len = last_line_text.len() as u32;

    let insert_text: String = missing
        .iter()
        .map(|s| format!("\n{}- [ ] \"{}\"", " ".repeat(node_indent), s))
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
        "Insert missing neighbor task(s)",
        uri,
        diagnostic,
        edit,
    ))
}

#[cfg(test)]
#[path = "missing_neighbor_tasks_tests.rs"]
mod tests;
