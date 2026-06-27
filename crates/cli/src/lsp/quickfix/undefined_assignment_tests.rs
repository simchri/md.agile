use super::super::{build_quickfix, build_quickfixes};
use tempfile::TempDir;
use tower_lsp::lsp_types::*;

fn diag_e009(line: u32, at_col: u32, assignment_name: &str) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: at_col.max(1),
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E009".into())),
        source: Some("agilels".into()),
        message: format!("undefined assignment: @{}", assignment_name),
        data: Some(serde_json::json!({
            "kind": "undefined_assignment",
            "assignment_name": assignment_name,
        })),
        ..Diagnostic::default()
    }
}

/// E009 quickfix offers "Add '[Users.X]'" AND "Add '[Groups.X]'" to toml.
#[test]
fn build_quickfix_e009_adds_user_and_group_actions() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    std::fs::write(project_dir.join("mdagile.toml"), "").unwrap();

    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "- [ ] task @alice\n";

    let diag = diag_e009(0, 11, "alice");
    let actions = build_quickfixes(&diag, doc, &uri);

    // No correction (empty toml), so exactly 2 add-to-toml actions.
    assert_eq!(
        actions.len(),
        2,
        "expected Users + Groups actions, got: {actions:?}"
    );
    assert!(
        actions.iter().any(|a| a.title.contains("[Users.alice]")),
        "should offer Add '[Users.alice]'"
    );
    assert!(
        actions.iter().any(|a| a.title.contains("[Groups.alice]")),
        "should offer Add '[Groups.alice]'"
    );
}

/// E009 quickfix adds `[Users.alice]` section to toml.
#[test]
fn build_quickfix_e009_user_section_content() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    std::fs::write(project_dir.join("mdagile.toml"), "").unwrap();

    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "- [ ] task @alice\n";
    let diag = diag_e009(0, 11, "alice");

    let actions = build_quickfixes(&diag, doc, &uri);
    let users_action = actions
        .iter()
        .find(|a| a.title.contains("[Users.alice]"))
        .unwrap();

    let toml_uri: Url = Url::from_file_path(project_dir.join("mdagile.toml")).unwrap();
    let edits = users_action
        .edit
        .as_ref()
        .and_then(|w| w.changes.as_ref())
        .and_then(|c| c.get(&toml_uri))
        .expect("edit targets mdagile.toml");

    assert!(edits[0].new_text.contains("[Users.alice]"));
}

/// E009 quickfix suggests a spelling correction when a close match exists.
#[test]
fn build_quickfixes_e009_suggests_typo_correction_from_users() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    // Known user is "alice"; typed "@alce" (one char off)
    std::fs::write(project_dir.join("mdagile.toml"), "[Users.alice]\n").unwrap();

    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "- [ ] task @alce\n";
    // "@" is at column 11 (0-indexed)
    let diag = diag_e009(0, 11, "alce");

    let actions = build_quickfixes(&diag, doc, &uri);

    // 1 correction + 2 add-to-toml = 3
    assert_eq!(
        actions.len(),
        3,
        "expected correction + Users + Groups, got: {actions:?}"
    );

    let correction = &actions[0];
    assert!(
        correction.title.contains("alice"),
        "first action is the spelling fix: {}",
        correction.title
    );
    assert_eq!(correction.is_preferred, Some(true));

    // The correction edit targets the .agile.md file
    let fix_edits = correction
        .edit
        .as_ref()
        .and_then(|w| w.changes.as_ref())
        .and_then(|c| c.get(&uri))
        .expect("correction targets agile.md");

    assert_eq!(fix_edits[0].new_text, "@alice");
    // range: col 11 .. 11 + 5 ("@alce")
    assert_eq!(fix_edits[0].range.start.character, 11);
    assert_eq!(fix_edits[0].range.end.character, 16);

    // Add-to-toml actions are deprioritised
    for add_action in &actions[1..] {
        assert_eq!(add_action.is_preferred, Some(false));
    }
}

/// E009 quickfix suggests correction from groups when close match found there.
#[test]
fn build_quickfixes_e009_suggests_typo_correction_from_groups() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    // Known group is "backend"; user typed "@backnd"
    std::fs::write(project_dir.join("mdagile.toml"), "[Groups.backend]\n").unwrap();

    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "- [ ] task @backnd\n";
    let diag = diag_e009(0, 11, "backnd");

    let actions = build_quickfixes(&diag, doc, &uri);

    let correction = actions.iter().find(|a| a.title.contains("backend"));
    assert!(
        correction.is_some(),
        "should find a correction towards 'backend'"
    );
}

/// E009: no correction offered when typed name is too different from all known.
#[test]
fn build_quickfixes_e009_no_correction_when_no_close_match() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    std::fs::write(project_dir.join("mdagile.toml"), "[Users.xyz]\n").unwrap();

    let tasks_file = project_dir.join("tasks.agile.md");
    let uri: Url = Url::from_file_path(&tasks_file).unwrap();
    let doc = "- [ ] task @completelydifferent\n";
    let diag = diag_e009(0, 11, "completelydifferent");

    let actions = build_quickfixes(&diag, doc, &uri);

    // Only the two add-to-toml actions; no correction
    assert_eq!(actions.len(), 2);
    assert!(actions.iter().all(|a| a.title.contains("Add")));
}
