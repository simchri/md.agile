use super::super::{build_quickfix, build_quickfixes};
use super::*;
use serde_json::json;
use tower_lsp::lsp_types::*;

fn diag_e010(line: u32, missing_tasks: Vec<&str>) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 1 },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E010".into())),
        source: Some("agilels".into()),
        message: "missing subtasks".into(),
        data: Some(json!({
            "kind": "missing_required_subtasks",
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
- [ ] #feature: add basket
";
    let diag = diag_e010(0, vec!["PO review"]);
    let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");
    assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    assert_eq!(action.is_preferred, Some(true));
}

#[test]
fn returns_none_when_missing_list_is_empty() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: add basket
";
    let diag = diag_e010(0, vec![]);
    assert!(build_quickfix(&diag, doc, &uri).is_none());
}

#[test]
fn returns_none_when_no_issue_data() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: add basket
";
    let mut diag = diag_e010(0, vec!["PO review"]);
    diag.data = None;
    assert!(build_quickfix(&diag, doc, &uri).is_none());
}

// ── no children: insert after task line ──────────────────────────────────────

#[test]
fn no_children_inserts_single_subtask_after_task_line() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: add basket
";
    let diag = diag_e010(0, vec!["PO review"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    let e = &edits[0];
    // Insertion point: end of task line (line 0)
    assert_eq!(
        e.range.start,
        Position {
            line: 0,
            character: 26
        }
    );
    assert_eq!(
        e.range.end,
        Position {
            line: 0,
            character: 26
        }
    );
    assert_eq!(e.new_text, "\n  - [ ] \"PO review\"");
}

#[test]
fn no_children_inserts_multiple_subtasks_after_task_line() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: add basket
";
    let diag = diag_e010(0, vec!["PO review", "dev implementation"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    let e = &edits[0];
    assert_eq!(
        e.new_text,
        "\n  - [ ] \"PO review\"\n  - [ ] \"dev implementation\""
    );
}

// ── with existing children: insert after last child ───────────────────────────

#[test]
fn with_one_child_inserts_after_it() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
";
    let diag = diag_e010(0, vec!["dev implementation"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    let e = &edits[0];
    // Last child is line 1 ("  - [ ] \"PO review\"" = 20 chars)
    assert_eq!(e.range.start.line, 1);
    assert_eq!(e.new_text, "\n  - [ ] \"dev implementation\"");
}

#[test]
fn with_multiple_children_inserts_after_last_child() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
  - [ ] some custom task
- [ ] unrelated task
";
    let diag = diag_e010(0, vec!["dev implementation"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    let e = &edits[0];
    // Last child belonging to line 0's subtree is line 2
    assert_eq!(e.range.start.line, 2);
    assert_eq!(e.new_text, "\n  - [ ] \"dev implementation\"");
}

#[test]
fn does_not_include_next_sibling_as_child() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: task A
- [ ] task B
";
    // Task A has no children; task B is a sibling at same indent.
    let diag = diag_e010(0, vec!["PO review"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    // Insertion is at end of line 0, not line 1
    assert_eq!(edits[0].range.start.line, 0);
}

// ── nested task (subtask with E010) ──────────────────────────────────────────

#[test]
fn nested_task_uses_correct_child_indent() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: add basket
  - [ ] \"developer #review\"
";
    // The subtask at line 1 (indent=2) is missing its required child.
    let diag = diag_e010(1, vec!["independent review"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    let e = &edits[0];
    // Insertion at end of line 1 (the subtask line itself has no children)
    assert_eq!(e.range.start.line, 1);
    // indent=2 → child_indent=4
    assert_eq!(e.new_text, "\n    - [ ] \"independent review\"");
}

#[test]
fn nested_task_with_existing_child_inserts_after_last_grandchild() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: add basket
  - [ ] \"developer #review\"
    - [ ] \"existing sub\"
";
    // The subtask at line 1 (indent=2) already has one child; missing another.
    let diag = diag_e010(1, vec!["independent review"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    let e = &edits[0];
    // Insertion after the grandchild on line 2
    assert_eq!(e.range.start.line, 2);
    assert_eq!(e.new_text, "\n    - [ ] \"independent review\"");
}

// ── edge cases ────────────────────────────────────────────────────────────────

#[test]
fn task_is_last_line_no_trailing_newline() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    // No trailing newline — doc ends right after the task.
    let doc = "- [ ] #feature: add basket";
    let diag = diag_e010(0, vec!["PO review"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].range.start.line, 0);
    assert_eq!(edits[0].new_text, "\n  - [ ] \"PO review\"");
}

#[test]
fn subtree_with_blank_line_between_children_inserts_after_true_last_child() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"

  - [ ] some custom subtask
- [ ] unrelated
";
    // The blank on line 2 is within the subtree; last real child is line 3.
    let diag = diag_e010(0, vec!["dev implementation"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits[0].range.start.line, 3);
}
