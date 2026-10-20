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
fn branch_form_quickfix_uses_only_active_field_defaults_and_multiline_settings() {
    let temp_dir = TempDir::new().unwrap();
    let config = "\
[Field.base]
label = \"Base\"
default = \"base default\"
[Field.active]
label = \"Details\"
default = \"active default\"
multiline = true
[Field.inactive]
label = \"Details\"
default = \"inactive default\"
[Form.base]
fields = [\"base\"]
[Form.active]
fields = [\"active\"]
[Form.inactive]
fields = [\"inactive\"]
[Properties.review]
form = \"base\"
[Properties.review.passed]
form = \"active\"
[Properties.review.failed]
form = \"inactive\"
";
    std::fs::write(temp_dir.path().join("mdagile.toml"), config).unwrap();
    let uri = Url::from_file_path(temp_dir.path().join("tasks.agile.md")).unwrap();
    let file_content = "\
- [ ] #review:passed
";
    let diagnostic = diag_e024(0, vec!["Base", "Details"]);

    let action = build_quickfix(&diagnostic, file_content, &uri).unwrap();

    let expected = format!(
        "\
{newline}  __Base:__ base default
  __Details:__
  active default",
        newline = '\n'
    );
    assert_eq!(get_edits(&action, &uri)[0].new_text, expected);
}

#[test]
fn branch_form_quickfix_inserts_a_repeated_missing_label_once() {
    let temp_dir = TempDir::new().unwrap();
    let config = "\
[Field.details]
label = \"Details\"
[Form.details]
fields = [\"details\"]
[Properties.review]
[Properties.review.passed]
form = \"details\"
";
    std::fs::write(temp_dir.path().join("mdagile.toml"), config).unwrap();
    let uri = Url::from_file_path(temp_dir.path().join("tasks.agile.md")).unwrap();
    let file_content = "\
- [ ] #review:passed
";
    let diagnostic = diag_e024(0, vec!["Details", "Details"]);

    let action = build_quickfix(&diagnostic, file_content, &uri).unwrap();

    let expected = format!(
        "\
{newline}  __Details:__ ___",
        newline = '\n'
    );
    assert_eq!(get_edits(&action, &uri)[0].new_text, expected);
}

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
fn a_multiline_field_with_a_newline_containing_default_is_inserted_using_the_multiline_grammar() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    let toml = "\
[Field.bio]
label = \"Bio\"
multiline = true
default = \"\"\"
line one
line two\"\"\"
";
    std::fs::write(project_dir.join("mdagile.toml"), toml).unwrap();
    let uri = Url::from_file_path(project_dir.join("tasks.agile.md")).unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec!["Bio"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    // Opens the label on its own line and places each default line below
    // it indented to match -- no closing `...` marker is needed since the
    // field is terminated by the next task or end of file. Re-parsing
    // this back recovers the configured default as the field's value.
    assert_eq!(edits[0].new_text, "\n  __Bio:__\n  line one\n  line two");
}

#[test]
fn a_multiline_field_with_no_default_is_inserted_blank() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    let toml = "\
[Field.bio]
label = \"Bio\"
multiline = true
";
    std::fs::write(project_dir.join("mdagile.toml"), toml).unwrap();
    let uri = Url::from_file_path(project_dir.join("tasks.agile.md")).unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec!["Bio"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    // No configured default -- open the field with nothing after it, same
    // as a plain blank multiline field (recognized as unfilled by E026,
    // same as the generic `___` placeholder is for a single-line field).
    assert_eq!(edits[0].new_text, "\n  __Bio:__");
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

#[test]
fn inserted_multiline_field_round_trips_through_the_parser_as_the_configured_default() {
    // Applying the quickfix's edit and re-parsing the result must recover
    // exactly the configured default as the field's parsed value, with
    // `multiline: true` -- i.e. the inserted text is genuinely valid
    // multiline syntax, not just visually similar to it.
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    let toml = "\
[Field.bio]
label = \"Bio\"
multiline = true
default = \"\"\"
line one
line two\"\"\"
";
    std::fs::write(project_dir.join("mdagile.toml"), toml).unwrap();
    let uri = Url::from_file_path(project_dir.join("tasks.agile.md")).unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec!["Bio"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    // The edit always appends after the last existing line's content, so
    // applying it is just string concatenation here.
    let updated_doc = format!("{}{}", doc.trim_end_matches('\n'), edits[0].new_text);

    let items = crate::parser::parse(&updated_doc, std::path::PathBuf::from("tasks.agile.md"));
    let crate::parser::FileItem::Task(task) = &items[0] else {
        panic!("expected a task");
    };
    assert_eq!(task.fields.len(), 1);
    assert!(task.fields[0].multiline);
    assert_eq!(task.fields[0].label, "Bio");
    // The parser captures multiline content verbatim, including each
    // line's indentation -- so the round-tripped value carries the same
    // leading indentation the quickfix inserted, not the raw unindented
    // `default` string from the TOML.
    assert_eq!(task.fields[0].value, "  line one\n  line two");
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

// ── config edge cases ────────────────────────────────────────────────────────

#[test]
fn malformed_toml_on_disk_falls_back_to_placeholder() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    // Invalid TOML (unterminated string) -- `read_toml` should fail to
    // parse it and the quickfix should degrade to the plain placeholder
    // rather than panicking or propagating the error.
    std::fs::write(project_dir.join("mdagile.toml"), "[Field.email\nlabel = ").unwrap();
    let uri = Url::from_file_path(project_dir.join("tasks.agile.md")).unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec!["Email"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits[0].new_text, "\n  __Email:__ ___");
}

#[test]
fn conflicting_config_files_on_disk_falls_back_to_placeholder() {
    // Both `mdagile.toml` and `.mdagile.toml` present is a conflicting-config
    // error from `config_source::resolve`; the quickfix must not use either
    // and should still produce the plain-placeholder action rather than
    // returning no quickfix at all.
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    let toml = "\
[Field.email]
label = \"Email\"
default = \"unknown@example.com\"
";
    std::fs::write(project_dir.join("mdagile.toml"), toml).unwrap();
    std::fs::write(project_dir.join(".mdagile.toml"), toml).unwrap();
    let uri = Url::from_file_path(project_dir.join("tasks.agile.md")).unwrap();
    let doc = "\
- [ ] #onboarding: add basket
";
    let diag = diag_e024(0, vec!["Email"]);
    let action = build_quickfix(&diag, doc, &uri).unwrap();
    let edits = get_edits(&action, &uri);

    assert_eq!(edits[0].new_text, "\n  __Email:__ ___");
}
