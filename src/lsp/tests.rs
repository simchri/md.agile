use super::*;

fn diag_e002(line: u32, current_indent: u32, expected_indent: usize) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end:   Position { line, character: current_indent.max(1) },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E002".into())),
        source: Some("agilels".into()),
        message: "wrong indent".into(),
        data: Some(serde_json::json!({
            "kind": "wrong_indent",
            "expected_indent": expected_indent,
        })),
        ..Diagnostic::default()
    }
}

#[test]
fn build_quickfix_replaces_three_space_indent_with_two() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] top
   - [ ] sub
";
    // Subtask is on line 1 (0-based) and currently has 3 spaces.
    let diag = diag_e002(1, 3, 2);

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
    assert_eq!(e.range.start, Position { line: 1, character: 0 });
    assert_eq!(e.range.end,   Position { line: 1, character: 3 });
    assert_eq!(e.new_text, "  ");
}

#[test]
fn build_quickfix_handles_deeper_subtask() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    // 5-space-indented level-2 subtask, expected = 4 spaces.
    let doc = "\
- [ ] top
  - [ ] mid
     - [ ] deep
";
    let diag = diag_e002(2, 5, 4);

    let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");
    let edits = action
        .edit
        .as_ref()
        .and_then(|w| w.changes.as_ref())
        .and_then(|c| c.get(&uri))
        .unwrap();
    assert_eq!(edits[0].range.end.character, 5);
    assert_eq!(edits[0].new_text, "    ");
}

#[test]
fn build_quickfix_returns_none_for_e001() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
  - [ ] orphan
";
    let diag = Diagnostic {
        range: Range {
            start: Position { line: 0, character: 0 },
            end:   Position { line: 0, character: 2 },
        },
        code: Some(NumberOrString::String("E001".into())),
        ..Diagnostic::default()
    };

    assert!(build_quickfix(&diag, doc, &uri).is_none());
}

#[test]
fn build_quickfix_returns_none_when_data_missing() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] top
   - [ ] sub
";
    let mut diag = diag_e002(1, 3, 2);
    diag.data = None;

    assert!(build_quickfix(&diag, doc, &uri).is_none());
}
