use super::super::build_quickfix;
use super::*;
use serde_json::json;
use tempfile::TempDir;

fn diag_e024(line: u32, missing_fields: Vec<&str>) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 1 },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E024".into())),
        source: Some("agilels".into()),
        message: "missing required field(s)".into(),
        data: Some(json!({
            "kind": "missing_required_field",
            "missing": missing_fields,
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

// ── basic shape (no config on disk) ─────────────────────────────────────────

#[test]
fn produces_a_quickfix_action() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec!["First name"]);
    let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");
    assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    assert_eq!(action.is_preferred, Some(true));
}

#[test]
fn returns_none_when_missing_list_is_empty() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec![]);
    assert!(build_quickfix(&diag, doc, &uri).is_none());
}

#[test]
fn returns_none_when_no_issue_data() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let mut diag = diag_e024(0, vec!["First name"]);
    diag.data = None;
    assert!(build_quickfix(&diag, doc, &uri).is_none());
}

#[test]
fn no_config_on_disk_inserts_generic_placeholder() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    let uri = Url::from_file_path(project_dir.join("tasks.agile.md")).unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec!["First name", "Email"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    assert_eq!(
        edits[0].new_text,
        "\n  __First name:__ ___\n  __Email:__ ___"
    );
}

// ── with config on disk ──────────────────────────────────────────────────────

#[test]
fn prefills_configured_default_value() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    let toml = "\
[Field.first_name]
label = \"First name\"

[Field.email]
label = \"Email\"
default = \"unknown@example.com\"
";
    std::fs::write(project_dir.join("mdagile.toml"), toml).unwrap();
    let uri = Url::from_file_path(project_dir.join("tasks.agile.md")).unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec!["First name", "Email"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    assert_eq!(
        edits[0].new_text,
        "\n  __First name:__ ___\n  __Email:__ unknown@example.com"
    );
}

#[test]
fn field_without_matching_label_falls_back_to_placeholder() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    let toml = "\
[Field.email]
label = \"Email\"
default = \"unknown@example.com\"
";
    std::fs::write(project_dir.join("mdagile.toml"), toml).unwrap();
    let uri = Url::from_file_path(project_dir.join("tasks.agile.md")).unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec!["Phone number"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits[0].new_text, "\n  __Phone number:__ ___");
}

#[test]
fn build_quickfix_e024_finds_toml_in_a_parent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    let toml = "\
[Field.email]
label = \"Email\"
default = \"unknown@example.com\"
";
    std::fs::write(project_dir.join("mdagile.toml"), toml).unwrap();

    let sub_dir = project_dir.join("tasks").join("current");
    std::fs::create_dir_all(&sub_dir).unwrap();
    let tasks_file = sub_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec!["Email"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits[0].new_text, "\n  __Email:__ unknown@example.com");
}

// ── with existing children/fields: insert after last subtree line ──────────

#[test]
fn with_existing_field_inserts_after_it() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #onboarding: add basket
  __First name:__ Jane
";
    let diag = diag_e024(0, vec!["Email"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].range.start.line, 1);
    assert_eq!(edits[0].new_text, "\n  __Email:__ ___");
}

#[test]
fn nested_subtask_uses_correct_field_indent() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ] #feature: add basket
  - [ ] \"developer #onboarding\"
";
    let diag = diag_e024(1, vec!["Email"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].range.start.line, 1);
    assert_eq!(edits[0].new_text, "\n    __Email:__ ___");
}
