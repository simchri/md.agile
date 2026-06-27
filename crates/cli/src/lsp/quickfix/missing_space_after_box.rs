use crate::rules::IssueData;
use tower_lsp::lsp_types::*;

/// E005: insert a space after the first `]` on the line.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let IssueData::MissingSpaceAfterBox = super::issue_data(diagnostic)? else {
        return None;
    };

    let line = diagnostic.range.start.line;
    let line_text = doc_text.lines().nth(line as usize)?;
    let after_bracket = (line_text.find(']')? + 1) as u32;

    let edit = TextEdit {
        range: Range {
            start: Position {
                line,
                character: after_bracket,
            },
            end: Position {
                line,
                character: after_bracket,
            },
        },
        new_text: " ".to_string(),
    };

    Some(super::make_quickfix(
        "Add space after status box",
        uri,
        diagnostic,
        edit,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::{build_quickfix, build_quickfixes};
    use super::*;
    use serde_json::json;
    use tower_lsp::lsp_types::*;

    fn diag_e005(line: u32) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position { line, character: 0 },
                end: Position { line, character: 1 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("E005".into())),
            source: Some("agilels".into()),
            message: "missing space after box".into(),
            data: Some(json!({
                "kind": "missing_space_after_box",
            })),
            ..Diagnostic::default()
        }
    }

    #[test]
    fn build_quickfix_e005_missing_space_after_box() {
        let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
        let doc = "\
- [ ]missing space
";
        let diag = diag_e005(0);

        let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");

        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        assert_eq!(action.is_preferred, Some(true));

        let edits = action
            .edit
            .as_ref()
            .and_then(|w| w.changes.as_ref())
            .and_then(|c| c.get(&uri))
            .expect("edit should target our uri");
        assert_eq!(edits.len(), 1);
        let e = &edits[0];
        // Position right after the `]` at position 5
        assert_eq!(
            e.range.start,
            Position {
                line: 0,
                character: 5
            }
        );
        assert_eq!(
            e.range.end,
            Position {
                line: 0,
                character: 5
            }
        );
        assert_eq!(e.new_text, " ");
    }

    #[test]
    fn build_quickfix_e005_returns_none_when_no_bracket() {
        let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
        let doc = "some text without bracket";

        let diag = diag_e005(0);

        assert!(build_quickfix(&diag, doc, &uri).is_none());
    }
}
