use tower_lsp::lsp_types::*;

/// E007: replace `[X]` with `[x]` by editing only the `X` character.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let line = diagnostic.range.start.line;
    let line_text = doc_text.lines().nth(line as usize)?;
    let x_col = (line_text.find("[X]")? + 1) as u32;

    let edit = TextEdit {
        range: Range {
            start: Position {
                line,
                character: x_col,
            },
            end: Position {
                line,
                character: x_col + 1,
            },
        },
        new_text: "x".to_string(),
    };

    Some(super::make_quickfix(
        "Replace [X] with [x]",
        uri,
        diagnostic,
        edit,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::{build_quickfix, build_quickfixes};
    use super::*;
    use tower_lsp::lsp_types::*;

    fn diag_e007(line: u32) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position { line, character: 0 },
                end: Position { line, character: 1 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("E007".into())),
            source: Some("agilels".into()),
            message: "uppercase X in status box".into(),
            ..Diagnostic::default()
        }
    }

    #[test]
    fn build_quickfix_e007_replaces_uppercase_x_with_lowercase() {
        let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
        let doc = "\
- [X] task
";
        let diag = diag_e007(0);

        let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));

        let edits = action
            .edit
            .as_ref()
            .and_then(|w| w.changes.as_ref())
            .and_then(|c| c.get(&uri))
            .expect("edit should target our uri");
        assert_eq!(edits.len(), 1);
        let e = &edits[0];
        // Replaces `X` (position 3..4) with `x`
        assert_eq!(
            e.range.start,
            Position {
                line: 0,
                character: 3
            }
        );
        assert_eq!(
            e.range.end,
            Position {
                line: 0,
                character: 4
            }
        );
        assert_eq!(e.new_text, "x");
    }

    #[test]
    fn build_quickfix_e007_returns_none_when_no_uppercase_x() {
        let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
        let doc = "- [x] task";
        let diag = diag_e007(0);
        assert!(build_quickfix(&diag, doc, &uri).is_none());
    }
}
