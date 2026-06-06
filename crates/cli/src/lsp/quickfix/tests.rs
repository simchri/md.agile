use super::*;
use serde_json::json;
use tower_lsp::lsp_types::*;

fn diag_e002(line: u32, current_indent: u32, expected_indent: usize) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: current_indent.max(1),
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E002".into())),
        source: Some("agilels".into()),
        message: "wrong indent".into(),
        data: Some(json!({
            "kind": "wrong_indent",
            "expected_indent": expected_indent,
        })),
        ..Diagnostic::default()
    }
}

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

fn diag_e005(line: u32) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 1 },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E005".into())),
        source: Some("agilels".into()),
        message: "missing space after box".into(),
        data: Some(json!({
            "kind": "missing_space_after_box",
        })),
        ..Diagnostic::default()
    }
}

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

fn diag_e007(line: u32) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 1 },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E007".into())),
        source: Some("agilels".into()),
        message: "uppercase X in status box".into(),
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
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 2,
            },
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

#[test]
fn build_quickfix_e005_missing_space_after_box() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [ ]missing space
";
    let diag = diag_e005(0);

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
    // Position right after the `]` at position 5
    assert_eq!(
        e.range.start,
        Position {
            line: 0,
            character: 5
        }
    );
    assert_eq!(
        e.range.end,
        Position {
            line: 0,
            character: 5
        }
    );
    assert_eq!(e.new_text, " ");
}

#[test]
fn build_quickfix_e005_returns_none_when_no_bracket() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "some text without bracket";

    let diag = diag_e005(0);

    assert!(build_quickfix(&diag, doc, &uri).is_none());
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

#[test]
fn build_quickfix_e007_replaces_uppercase_x_with_lowercase() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "\
- [X] task
";
    let diag = diag_e007(0);

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
    // Replaces `X` (position 3..4) with `x`
    assert_eq!(
        e.range.start,
        Position {
            line: 0,
            character: 3
        }
    );
    assert_eq!(
        e.range.end,
        Position {
            line: 0,
            character: 4
        }
    );
    assert_eq!(e.new_text, "x");
}

#[test]
fn build_quickfix_e007_returns_none_when_no_uppercase_x() {
    let uri: Url = "file:///tmp/example.agile.md".parse().unwrap();
    let doc = "- [x] task";
    let diag = diag_e007(0);
    assert!(build_quickfix(&diag, doc, &uri).is_none());
}

#[test]
fn build_quickfix_e008_adds_property_to_empty_toml() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create an empty mdagile.toml
    std::fs::write(project_dir.join("mdagile.toml"), "").unwrap();

    // Create a tasks file in the project directory
    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "\
- [ ] task #undefined
";

    // Create a diagnostic for E008 with property name in data
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

    // The quickfix should target mdagile.toml
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
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create mdagile.toml with existing properties
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
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Existing property is "feature"; user typed "feture" (one char deleted)
    std::fs::write(project_dir.join("mdagile.toml"), "[Properties.feature]\n").unwrap();

    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    // "- [ ] task #feture"
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

    // Must have exactly 2 actions: add-to-toml + correct-spelling
    assert_eq!(
        actions.len(),
        2,
        "expected add+correct actions, got: {actions:?}"
    );

    // First action: add to mdagile.toml
    let add_action = &actions[0];
    assert!(
        add_action.title.contains("Add"),
        "first action should be 'Add …': {}",
        add_action.title
    );

    // Second action: correct spelling in the document
    let fix_action = &actions[1];
    assert!(
        fix_action.title.contains("feature"),
        "correction action title should mention 'feature': {}",
        fix_action.title
    );
    assert_eq!(fix_action.kind, Some(CodeActionKind::QUICKFIX));

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
}

/// No correction is offered when the typed name is too different from every
/// known property.
#[test]
fn build_quickfixes_e008_no_correction_when_no_close_match() {
    use tempfile::TempDir;

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

#[test]
fn has_quickfix_for_each_code() {
    use crate::rules::ErrorCode::*;
    assert!(!has_quickfix(OrphanedSubtask));
    assert!(has_quickfix(WrongIndentation));
    assert!(has_quickfix(WrongBodyIndentation));
    assert!(!has_quickfix(IncompleteParent));
    assert!(has_quickfix(MissingSpaceAfterBox));
    assert!(has_quickfix(BoxStyleInvalid));
    assert!(has_quickfix(UppercaseX));
    assert!(has_quickfix(UndefinedProperty));
}
