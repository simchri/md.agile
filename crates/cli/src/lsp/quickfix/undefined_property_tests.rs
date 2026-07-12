use super::super::{build_quickfix, build_quickfixes};
use tempfile::TempDir;
use tower_lsp::lsp_types::*;

#[test]
fn build_quickfix_e008_finds_toml_in_a_parent_directory() {
    // The task file lives in a subdirectory; mdagile.toml lives in the
    // project root above it — the toml lookup must walk up to find it.
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    std::fs::write(project_dir.join("mdagile.toml"), "").unwrap();

    let sub_dir = project_dir.join("tasks").join("current");
    std::fs::create_dir_all(&sub_dir).unwrap();
    let tasks_file = sub_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "\
- [ ] task #undefined
";

    let diag = Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 1,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E008".into())),
        source: Some("agilels".into()),
        message: "undefined property".into(),
        data: Some(serde_json::json!({
            "kind": "undefined_property",
            "property_name": "undefined",
        })),
        ..Diagnostic::default()
    };

    let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");

    let toml_uri: Url = Url::from_file_path(project_dir.join("mdagile.toml")).unwrap();
    let changes = action
        .edit
        .as_ref()
        .and_then(|w| w.changes.as_ref())
        .expect("should have changes");

    assert!(
        changes.contains_key(&toml_uri),
        "should find mdagile.toml in the project root, not the task's own directory"
    );
}

#[test]
fn build_quickfix_e008_adds_property_to_empty_toml() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    std::fs::write(project_dir.join("mdagile.toml"), "").unwrap();

    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "\
- [ ] task #undefined
";

    let diag = Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 1,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E008".into())),
        source: Some("agilels".into()),
        message: "undefined property".into(),
        data: Some(serde_json::json!({
            "kind": "undefined_property",
            "property_name": "undefined",
        })),
        ..Diagnostic::default()
    };

    let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");

    assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    assert_eq!(action.is_preferred, Some(true));

    let toml_uri: Url = Url::from_file_path(project_dir.join("mdagile.toml")).unwrap();
    let changes = action
        .edit
        .as_ref()
        .and_then(|w| w.changes.as_ref())
        .expect("should have changes");

    assert!(
        changes.contains_key(&toml_uri),
        "should have changes for mdagile.toml"
    );

    let edits = &changes[&toml_uri];
    assert_eq!(edits.len(), 1);
    assert!(edits[0].new_text.contains("[Properties.undefined]"));
}

#[test]
fn build_quickfix_e008_appends_to_existing_toml() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    std::fs::write(
        project_dir.join("mdagile.toml"),
        "[Properties.feature]\n[Properties.bug]\n",
    )
    .unwrap();

    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "- [ ] task #newprop\n";

    let diag = Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 1,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E008".into())),
        source: Some("agilels".into()),
        message: "undefined property".into(),
        data: Some(serde_json::json!({
            "kind": "undefined_property",
            "property_name": "newprop",
        })),
        ..Diagnostic::default()
    };

    let action = build_quickfix(&diag, doc, &uri).expect("should produce a quickfix");

    let toml_uri: Url = Url::from_file_path(project_dir.join("mdagile.toml")).unwrap();
    let edits = action
        .edit
        .as_ref()
        .and_then(|w| w.changes.as_ref())
        .and_then(|c| c.get(&toml_uri))
        .expect("should have changes for mdagile.toml");

    let new_content = &edits[0].new_text;
    assert!(new_content.contains("[Properties.feature]"));
    assert!(new_content.contains("[Properties.bug]"));
    assert!(new_content.contains("[Properties.newprop]"));
}

/// `build_quickfixes` returns both an "add to toml" action AND a "correct
/// typo" action when the typed property closely matches an existing one.
#[test]
fn build_quickfixes_e008_suggests_typo_correction() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Existing property is "feature"; user typed "feture" (one char deleted)
    std::fs::write(project_dir.join("mdagile.toml"), "[Properties.feature]\n").unwrap();

    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    // "#" sits at character 11 (0-indexed): "- [ ] task " = 11 chars
    let doc = "- [ ] task #feture\n";

    let diag = Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            // end.character = 11 = 0-indexed column of '#'
            end: Position {
                line: 0,
                character: 11,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E008".into())),
        source: Some("agilels".into()),
        message: "undefined property".into(),
        data: Some(serde_json::json!({
            "kind": "undefined_property",
            "property_name": "feture",
        })),
        ..Diagnostic::default()
    };

    let actions = build_quickfixes(&diag, doc, &uri);

    // Must have exactly 2 actions: correct-spelling (preferred) + add-to-toml
    assert_eq!(
        actions.len(),
        2,
        "expected correct+add actions, got: {actions:?}"
    );

    // First action: correct spelling in the document (preferred)
    let fix_action = &actions[0];
    assert!(
        fix_action.title.contains("feature"),
        "first action should be the spelling correction mentioning 'feature': {}",
        fix_action.title
    );
    assert_eq!(fix_action.kind, Some(CodeActionKind::QUICKFIX));
    assert_eq!(
        fix_action.is_preferred,
        Some(true),
        "correction should be marked is_preferred"
    );

    // The correction edit must target the .agile.md file (not toml)
    let fix_edits = fix_action
        .edit
        .as_ref()
        .and_then(|w| w.changes.as_ref())
        .and_then(|c| c.get(&uri))
        .expect("correction action should edit the agile.md file");

    assert_eq!(fix_edits.len(), 1);
    let e = &fix_edits[0];
    // Replace "#feture" (7 chars) starting at column 11
    assert_eq!(
        e.range.start,
        Position {
            line: 0,
            character: 11
        }
    );
    assert_eq!(
        e.range.end,
        Position {
            line: 0,
            character: 18
        }
    ); // 11 + 7
    assert_eq!(e.new_text, "#feature");

    // Second action: add to mdagile.toml (not preferred when correction exists)
    let add_action = &actions[1];
    assert!(
        add_action.title.contains("Add"),
        "second action should be 'Add …': {}",
        add_action.title
    );
    assert_eq!(
        add_action.is_preferred,
        Some(false),
        "add-to-toml should not be preferred when a correction is available"
    );
}

/// No correction is offered when the typed name is too different from every
/// known property.
#[test]
fn build_quickfixes_e008_no_correction_when_no_close_match() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    std::fs::write(project_dir.join("mdagile.toml"), "[Properties.xyz]\n").unwrap();

    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "- [ ] task #completelydifferent\n";

    let diag = Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 11,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E008".into())),
        source: Some("agilels".into()),
        message: "undefined property".into(),
        data: Some(serde_json::json!({
            "kind": "undefined_property",
            "property_name": "completelydifferent",
        })),
        ..Diagnostic::default()
    };

    let actions = build_quickfixes(&diag, doc, &uri);

    // Only the "add to toml" action, no correction
    assert_eq!(actions.len(), 1);
    assert!(actions[0].title.contains("Add"));
}
