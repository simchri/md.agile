//! Form validation scripts (E030/E031) run in the language server only on
//! `textDocument/didSave`, never on open or on edits.

use super::helpers::{LspSession, file_uri};
use crate::helpers::run_agile_clean_env;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

struct Project {
    dir: TempDir,
    config_home: TempDir,
    uri: String,
}

const TASK_TEXT: &str = "\
- [ ] new colleague #onboarding
  __Email:__ john
";

fn project(trusted: bool) -> Project {
    let dir = tempfile::tempdir().unwrap();
    let config_home = tempfile::tempdir().unwrap();
    let config = "\
[Field.email]
label = \"Email\"

[Form.personal_info]
fields = [\"email\"]
validation = \"validate.sh\"

[Properties.onboarding]
form = \"personal_info\"
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false, \"errors\": [{\"field\": \"Email\", \"message\": \"not an email\"}]}'
";
    let script = dir.path().join("validate.sh");
    fs::write(&script, script_content).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(dir.path().join("tasks.agile.md"), TASK_TEXT).unwrap();
    if trusted {
        let out = run_agile_clean_env(
            dir.path(),
            &["trust", "--yes"],
            &[("XDG_CONFIG_HOME", config_home.path().to_str().unwrap())],
        );
        assert_eq!(out.status.code(), Some(0), "{out:?}");
    }
    let uri = file_uri(&dir.path().join("tasks.agile.md"));
    Project {
        dir,
        config_home,
        uri,
    }
}

fn start(project: &Project) -> LspSession {
    let root_uri = file_uri(project.dir.path());
    LspSession::start_with_root_uri_and_env(
        Some(&root_uri),
        &[(
            "XDG_CONFIG_HOME",
            project.config_home.path().to_str().unwrap(),
        )],
    )
}

fn codes(notification: &Value) -> Vec<String> {
    notification["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|d| d["code"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn scripts_do_not_run_on_open_but_do_on_save() {
    // Arrange
    let project = project(true);
    let mut session = start(&project);

    // Act
    session.open_document(&project.uri, TASK_TEXT);
    let on_open = session.read_notification("textDocument/publishDiagnostics");
    session.save_document(&project.uri);
    let on_save = session.read_notification("textDocument/publishDiagnostics");

    // Assert
    assert!(!codes(&on_open).contains(&"E030".to_string()), "{on_open}");
    assert!(codes(&on_save).contains(&"E030".to_string()), "{on_save}");
    let diagnostic = &on_save["params"]["diagnostics"][0];
    assert_eq!(diagnostic["range"]["start"]["line"], 1);
}

#[test]
fn editing_clears_the_script_diagnostics_until_the_next_save() {
    // Arrange
    let project = project(true);
    let mut session = start(&project);
    session.open_document(&project.uri, TASK_TEXT);
    session.read_notification("textDocument/publishDiagnostics");
    session.save_document(&project.uri);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    session.change_document(&project.uri, 2, TASK_TEXT);
    let on_change = session.read_notification("textDocument/publishDiagnostics");

    // Assert
    assert!(
        !codes(&on_change).contains(&"E030".to_string()),
        "{on_change}"
    );
}

#[test]
fn an_untrusted_script_is_reported_as_e031_on_save() {
    // Arrange
    let project = project(false);
    let mut session = start(&project);
    session.open_document(&project.uri, TASK_TEXT);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    session.save_document(&project.uri);
    let on_save = session.read_notification("textDocument/publishDiagnostics");

    // Assert
    assert!(codes(&on_save).contains(&"E031".to_string()), "{on_save}");
}
