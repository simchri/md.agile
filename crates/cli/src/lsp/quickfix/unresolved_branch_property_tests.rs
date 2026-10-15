use super::super::{build_quickfix, build_quickfixes};
use tempfile::TempDir;
use tower_lsp::lsp_types::*;

fn diag_e021(line: u32, properties: Vec<&str>, message: &str) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 1 },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E021".into())),
        source: Some("agilels".into()),
        message: message.into(),
        data: Some(serde_json::json!({
            "kind": "unresolved_branch_property",
            "properties": properties,
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

fn project(config: &str) -> (TempDir, Url) {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("mdagile.toml"), config).unwrap();
    let uri = Url::from_file_path(temp_dir.path().join("tasks.agile.md")).unwrap();
    (temp_dir, uri)
}

#[test]
fn offers_one_action_per_declared_branch() {
    let config = "\
[Properties.review]
[Properties.review.passed]
[Properties.review.failed]
";
    let (_dir, uri) = project(config);
    let doc = "\
- [x] perform #review:...
";
    let diag = diag_e021(
        0,
        vec!["review"],
        "Task marked done while branch property #review:... still unresolved",
    );

    let actions = build_quickfixes(&diag, doc, &uri);

    assert_eq!(actions.len(), 2);
    let new_texts: Vec<_> = actions
        .iter()
        .map(|action| get_edits(action, &uri)[0].new_text.clone())
        .collect();
    assert_eq!(new_texts, vec!["#review:failed", "#review:passed"]);
}

#[test]
fn replaces_only_the_pending_marker_text() {
    let config = "\
[Properties.review]
[Properties.review.passed]
";
    let (_dir, uri) = project(config);
    let doc = "\
- [x] perform #review:...
";
    let diag = diag_e021(
        0,
        vec!["review"],
        "Task marked done while branch property #review:... still unresolved",
    );

    let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    let edit = &edits[0];
    assert_eq!(edit.new_text, "#review:passed");
    assert_eq!(
        edit.range.start,
        Position {
            line: 0,
            character: 14
        }
    );
    assert_eq!(
        edit.range.end,
        Position {
            line: 0,
            character: 25
        }
    );
}

#[test]
fn handles_multiple_pending_properties_on_the_same_line_independently() {
    let config = "\
[Properties.review]
[Properties.review.passed]
[Properties.review.failed]

[Properties.approval]
[Properties.approval.granted]
";
    let (_dir, uri) = project(config);
    let doc = "\
- [x] perform #review:... and #approval:...
";
    let diag = diag_e021(
        0,
        vec!["review", "approval"],
        "Task marked done while branch properties #review:..., #approval:... still unresolved",
    );

    let actions = build_quickfixes(&diag, doc, &uri);

    let new_texts: Vec<_> = actions
        .iter()
        .map(|action| get_edits(action, &uri)[0].new_text.clone())
        .collect();
    assert_eq!(
        new_texts,
        vec!["#review:failed", "#review:passed", "#approval:granted"]
    );
}

#[test]
fn returns_empty_when_no_issue_data() {
    let config = "\
[Properties.review]
[Properties.review.passed]
";
    let (_dir, uri) = project(config);
    let doc = "\
- [x] perform #review:...
";
    let mut diag = diag_e021(
        0,
        vec!["review"],
        "Task marked done while branch property #review:... still unresolved",
    );
    diag.data = None;

    assert!(build_quickfixes(&diag, doc, &uri).is_empty());
}

#[test]
fn returns_empty_when_property_is_no_longer_in_config() {
    let config = "\
[Properties.feature]
";
    let (_dir, uri) = project(config);
    let doc = "\
- [x] perform #review:...
";
    let diag = diag_e021(
        0,
        vec!["review"],
        "Task marked done while branch property #review:... still unresolved",
    );

    assert!(build_quickfixes(&diag, doc, &uri).is_empty());
}
