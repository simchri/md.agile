use super::super::build_quickfix;
use super::*;
use serde_json::json;

fn diag_e019(line: u32, missing_tasks: Vec<&str>) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 1 },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E019".into())),
        source: Some("agilels".into()),
        message: "missing neighbor task".into(),
        data: Some(json!({
            "kind": "missing_neighbor_task",
            "missing": missing_tasks,
        })),
        ..Diagnostic::default()
    }
}

fn get_edits(action: &CodeAction, uri: &Url) -> Vec<TextEdit> {
    action
        .edit
        .as_ref()
        .and_then(|w| w.changes.as_ref())
        .and_then(|c| c.get(uri))
        .cloned()
        .expect("edit should target our uri")
}

// ── basic shape ───────────────────────────────────────────────────────────────

#[test]
fn produces_a_quickfix_action() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    let diag = diag_e019(1, vec!["do the back end implementation also!"]);
    let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");
    assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    assert_eq!(action.is_preferred, Some(true));
}

#[test]
fn returns_none_when_missing_list_is_empty() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    let diag = diag_e019(1, vec![]);
    assert!(build_quickfix(&diag, doc, &uri).is_none());
}

#[test]
fn returns_none_when_no_issue_data() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    let mut diag = diag_e019(1, vec!["do the back end implementation also!"]);
    diag.data = None;
    assert!(build_quickfix(&diag, doc, &uri).is_none());
}

// ── top-level: no quickfix possible ────────────────────────────────────────────

#[test]
fn returns_none_when_flagged_node_is_top_level() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #frontend-implementation: top level feature
";
    let diag = diag_e019(0, vec!["do the back end implementation also!"]);
    assert!(build_quickfix(&diag, doc, &uri).is_none());
}

// ── inserts as a sibling (not a child) ─────────────────────────────────────────

#[test]
fn no_siblings_inserts_single_neighbor_task_after_node_line() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    let diag = diag_e019(1, vec!["do the back end implementation also!"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    let e = &edits[0];
    let line_len = doc.lines().nth(1).unwrap().len() as u32;
    assert_eq!(
        e.range.start,
        Position {
            line: 1,
            character: line_len
        }
    );
    assert_eq!(
        e.range.end,
        Position {
            line: 1,
            character: line_len
        }
    );
    // Inserted at the *same* indentation as the flagged node (2 spaces),
    // not indented further as a child would be.
    assert_eq!(
        e.new_text,
        "\n  - [ ] \"do the back end implementation also!\""
    );
}

#[test]
fn inserts_multiple_neighbor_tasks_in_one_edit() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    let diag = diag_e019(1, vec!["backend", "QA sign-off"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    assert_eq!(
        edits[0].new_text,
        "\n  - [ ] \"backend\"\n  - [ ] \"QA sign-off\""
    );
}

#[test]
fn with_own_children_inserts_after_the_whole_subtree_not_inside_it() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
    - [ ] some child of the flagged subtask
";
    let diag = diag_e019(1, vec!["do the back end implementation also!"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    // Insertion point is after line 2 (the flagged node's own child), not
    // right after line 1 -- the neighbor task must come after the whole
    // subtree, still as a sibling of the flagged node.
    assert_eq!(edits[0].range.start.line, 2);
    assert_eq!(
        edits[0].new_text,
        "\n  - [ ] \"do the back end implementation also!\""
    );
}

#[test]
fn with_existing_sibling_inserts_right_after_the_flagged_nodes_own_subtree() {
    // The insertion point is the end of the flagged node's own subtree (here,
    // just its own line, since it has no children) -- not after every later
    // sibling. Where exactly among siblings the new line lands doesn't matter
    // semantically, since Neighbor Tasks aren't ordered relative to each other.
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
  - [ ] some unrelated custom sibling
";
    let diag = diag_e019(1, vec!["do the back end implementation also!"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].range.start.line, 1);
}

// ── edge cases ────────────────────────────────────────────────────────────────

#[test]
fn flagged_node_is_last_line_no_trailing_newline() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation";
    let diag = diag_e019(1, vec!["do the back end implementation also!"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].range.start.line, 1);
    assert_eq!(
        edits[0].new_text,
        "\n  - [ ] \"do the back end implementation also!\""
    );
}

#[test]
fn nested_property_uses_correct_sibling_indent() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] outer task
  - [ ] \"PO review\"
  - [ ] ..first do the #frontend-implementation
";
    // The subtask at line 2 (indent=2) is missing its required neighbor.
    let diag = diag_e019(2, vec!["do the back end implementation also!"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].range.start.line, 2);
    // indent=2 → sibling also at indent=2 (not 4, which would be a child)
    assert_eq!(
        edits[0].new_text,
        "\n  - [ ] \"do the back end implementation also!\""
    );
}
