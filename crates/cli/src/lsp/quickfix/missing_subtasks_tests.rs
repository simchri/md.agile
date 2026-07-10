use super::super::{build_quickfix, build_quickfixes};
use super::*;
use tower_lsp::lsp_types::*;
use serde_json::json;

fn diag_e011(line: u32, missing_tasks: Vec<String>) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 1 },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E010".into())),
        source: Some("agilels".into()),
        message: "missing subtasks".into(),
        data: Some(json!({ "missing": missing_tasks })),
        ..Diagnostic::default()
    }
}

#[test]
fn build_quickfix_e011_builds_a_quickfix() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [X] task #feature
";
    let v: Vec<String> = vec![];
    let diag = diag_e011(0, v);

    let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");

    assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
}

#[test_log::test]
fn build_quickfix_e011_adds_subtasks_from_issue_payload() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [X] task #feature
";

    let v: Vec<String> = vec!["some task".into()];
    let diag = diag_e011(0, v);

    let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");

    let edits = action
        .edit
        .as_ref()
        .and_then(|w| w.changes.as_ref())
        .and_then(|c| c.get(&uri))
        .expect("edit should target our uri");

    assert_eq!(edits.len(), 1);
    let e = &edits[0];
    // 
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
            character: 1
        }
    );
    assert_eq!(e.new_text, "some task");
}
