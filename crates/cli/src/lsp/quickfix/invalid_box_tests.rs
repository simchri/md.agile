use super::super::build_quickfix;
use super::*;

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
