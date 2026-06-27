use super::super::{build_quickfix, build_quickfixes};
use super::*;
use serde_json::json;
use tower_lsp::lsp_types::*;

fn diag_e003(line: u32, current_indent: u32, expected_indent: usize) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: current_indent.max(1),
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E003".into())),
        source: Some("agilels".into()),
        message: "wrong body indent".into(),
        data: Some(json!({
            "kind": "wrong_body_indent",
            "expected_indent": expected_indent,
        })),
        ..Diagnostic::default()
    }
}

#[test]
fn build_quickfix_e003_wrong_body_indent() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] task title
   description line with extra space
";
    // Body line is on line 1 and currently has 3 spaces, expects 2.
    let diag = diag_e003(1, 3, 2);

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
    assert_eq!(
        e.range.start,
        Position {
            line: 1,
            character: 0
        }
    );
    assert_eq!(
        e.range.end,
        Position {
            line: 1,
            character: 3
        }
    );
    assert_eq!(e.new_text, "  ");
}
