use crate::rules::IssueData;
use tower_lsp::lsp_types::*;

/// E010: insert each missing required subtask as a new child line after the
/// last existing child of the flagged task (or directly after the task line
/// when it has no children yet).
///
/// Each inserted line is indented at `task_indent + 2` and formatted as
/// `- [ ] "subtask name"` (quoted, matching the PropertyRequired convention).
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let IssueData::MissingRequiredSubtasks { missing } = super::issue_data(diagnostic)? else {
        return None;
    };

    if missing.is_empty() {
        return None;
    }

    let task_line = diagnostic.range.start.line;
    let lines: Vec<&str> = doc_text.lines().collect();
    let task_line_text = lines.get(task_line as usize)?;
    let task_indent = task_line_text.chars().take_while(|c| *c == ' ').count();
    let child_indent = task_indent + 2;

    let last_line = find_last_subtree_line(&lines, task_line, task_indent);
    let last_line_text = lines.get(last_line as usize)?;
    let last_line_len = last_line_text.len() as u32;

    let insert_text: String = missing
        .iter()
        .map(|s| format!("\n{}- [ ] \"{}\"", " ".repeat(child_indent), s))
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
        "Insert missing required subtasks",
        uri,
        diagnostic,
        edit,
    ))
}

/// Returns the 0-based line index of the last line that belongs to the subtree
/// rooted at `task_line`. A line belongs to the subtree if it is blank or has
/// indentation strictly greater than `task_indent`. The scan stops at the first
/// non-blank line whose indentation is ≤ `task_indent`.
fn find_last_subtree_line(lines: &[&str], task_line: u32, task_indent: usize) -> u32 {
    let mut last = task_line;
    let mut i = task_line + 1;
    while (i as usize) < lines.len() {
        let line = lines[i as usize];
        if line.trim().is_empty() {
            // Blank line: keep scanning but don't extend `last` yet —
            // a trailing blank before the next sibling shouldn't be included.
            i += 1;
            continue;
        }
        let indent = line.chars().take_while(|c| *c == ' ').count();
        if indent > task_indent {
            last = i;
            i += 1;
        } else {
            break;
        }
    }
    last
}

#[cfg(test)]
#[path = "missing_subtasks_tests.rs"]
mod tests;
