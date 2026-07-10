use super::super::build_quickfix;
use super::*;

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
