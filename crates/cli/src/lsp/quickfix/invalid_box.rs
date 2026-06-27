use tower_lsp::lsp_types::*;

/// E006: replace an invalid `[…]` (e.g. `[]`, `[o]`, `[xx]`) with `[ ]`.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let line = diagnostic.range.start.line;
    let line_text = doc_text.lines().nth(line as usize)?;

    let open = line_text.find('[')?;
    let close = open + 1 + line_text[open + 1..].find(']')?;

    let edit = TextEdit {
        range: Range {
            start: Position {
                line,
                character: open as u32,
            },
            end: Position {
                line,
                character: (close + 1) as u32,
            },
        },
        new_text: "[ ]".to_string(),
    };

    Some(super::make_quickfix(
        "Replace with empty box: [ ]",
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

    fn diag_e006(line: u32) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position { line, character: 0 },
                end: Position { line, character: 1 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("E006".into())),
            source: Some("agilels".into()),
            message: "box style invalid".into(),
            ..Diagnostic::default()
        }
    }

    #[test]
    fn build_quickfix_e006_replaces_empty_box_with_todo() {
        let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
        let doc = "\
- [] task
";
        let diag = diag_e006(0);

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
        // Replaces `[]` (positions 2..4) with `[ ]`
        assert_eq!(
            e.range.start,
            Position {
                line: 0,
                character: 2
            }
        );
        assert_eq!(
            e.range.end,
            Position {
                line: 0,
                character: 4
            }
        );
        assert_eq!(e.new_text, "[ ]");
    }

    #[test]
    fn build_quickfix_e006_replaces_wrong_char_box_with_todo() {
        let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
        let doc = "\
- [o] task
";
        let diag = diag_e006(0);

        let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");
        let edits = action
            .edit
            .as_ref()
            .and_then(|w| w.changes.as_ref())
            .and_then(|c| c.get(&uri))
            .unwrap();
        let e = &edits[0];
        // Replaces `[o]` (positions 2..5) with `[ ]`
        assert_eq!(e.range.start.character, 2);
        assert_eq!(e.range.end.character, 5);
        assert_eq!(e.new_text, "[ ]");
    }

    #[test]
    fn build_quickfix_e006_returns_none_when_no_brackets() {
        let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
        let doc = "no brackets here";
        let diag = diag_e006(0);
        assert!(build_quickfix(&diag, doc, &uri).is_none());
    }
}
