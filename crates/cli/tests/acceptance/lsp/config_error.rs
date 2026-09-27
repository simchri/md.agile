use super::helpers::{LspSession, file_uri, start_project_session};

#[test]
fn lsp_revalidates_when_disk_config_contents_change() {
    // Arrange
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("mdagile.toml");
    let mut file_content = "\
[Properties.feature]
";
    std::fs::write(&config_path, file_content).unwrap();
    file_content = "\
- [ ] task #feature
";
    let uri = file_uri(&dir.path().join("tasks.agile.md"));
    let root_uri = file_uri(dir.path());
    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(&uri, file_content);
    let initial = session.read_notification("textDocument/publishDiagnostics");
    assert!(
        initial["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // Act
    file_content = "\
[Properties.other]
";
    std::fs::write(&config_path, file_content).unwrap();
    let changed = session.read_notification("textDocument/publishDiagnostics");

    // Assert
    assert!(
        changed["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E008")
    );
}

#[test]
fn lsp_revalidates_when_disk_config_is_created_and_removed_without_root() {
    // Arrange
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("mdagile.toml");
    let file_uri = file_uri(&dir.path().join("tasks.agile.md"));
    let mut file_content = "\
- [ ] task #feature
";
    let mut session = LspSession::start();
    session.open_document(&file_uri, file_content);
    let initial = session.read_notification("textDocument/publishDiagnostics");
    assert!(
        initial["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E008")
    );

    // Act
    file_content = "\
[Properties.feature]
";
    std::fs::write(&config_path, file_content).unwrap();
    let added = session.read_notification("textDocument/publishDiagnostics");
    std::fs::remove_file(&config_path).unwrap();
    let removed = session.read_notification("textDocument/publishDiagnostics");

    // Assert
    assert!(
        added["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        removed["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E008")
    );
}

#[test]
fn lsp_revalidates_when_disk_config_filename_switches() {
    // Arrange
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("mdagile.toml");
    let second = dir.path().join(".mdagile.toml");
    let mut file_content = "\
[Properties.feature]
";
    std::fs::write(&first, file_content).unwrap();
    file_content = "\
- [ ] task #feature #other
";
    let uri = file_uri(&dir.path().join("tasks.agile.md"));
    let root_uri = file_uri(dir.path());
    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(&uri, file_content);
    let initial = session.read_notification("textDocument/publishDiagnostics");
    assert!(
        initial["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("#other"))
    );

    // Act
    std::fs::remove_file(first).unwrap();
    file_content = "\
[Properties.other]
";
    std::fs::write(&second, file_content).unwrap();
    let changed = session.read_notification("textDocument/publishDiagnostics");
    session.send(
        &serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "textDocument/codeAction",
            "params": {
                "textDocument": {"uri": uri},
                "range": {"start": {"line": 0, "character": 0},
                          "end": {"line": 0, "character": 30}},
                "context": {"diagnostics": []}
            }
        })
        .to_string(),
    );
    let actions = session.read_response(2);

    // Assert
    let diagnostics = changed["params"]["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("#feature")),
        "{diagnostics:?}"
    );
    assert!(
        !diagnostics
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("#other"))
    );
    let new_uri = file_uri(&second);
    assert!(
        actions["result"].as_array().unwrap().iter().any(|action| {
            action["title"]
                .as_str()
                .unwrap_or("")
                .contains("[Properties.feature]")
                && action["edit"]["changes"][&new_uri].is_array()
        }),
        "{actions:?}"
    );
}

#[test]
fn lsp_diagnostics_follow_unsaved_config_and_report_invalid_changes() {
    // Arrange
    let config = "\
[Properties.feature]
";
    let file_content = "\
- [ ] task #feature
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");
    let config_uri = file_uri.replace("tasks.agile.md", "mdagile.toml");
    let unsaved_config = "\
[Properties.other]
";
    session.open_document(&config_uri, unsaved_config);

    // Act
    let updated = session.read_notification("textDocument/publishDiagnostics");

    // Assert
    assert!(
        updated["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E008")
    );

    let invalid_config = "\
[Properties.
";
    session.send(
        &serde_json::json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {"textDocument": {"uri": config_uri, "version": 2},
                       "contentChanges": [{"text": invalid_config}]}
        })
        .to_string(),
    );
    session.read_notification("window/showMessage");
    let broken = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = broken["params"]["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d["message"].as_str().unwrap_or("").contains("config error"))
    );
    assert!(!diagnostics.iter().any(|d| d["code"] == "E008"));
}

#[test]
fn lsp_reports_conflict_with_new_unsaved_config_buffer() {
    // Arrange
    let config = "\
[Properties.feature]
";
    let file_content = "\
- [ ] task #feature
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");
    let second_uri = file_uri.replace("tasks.agile.md", ".mdagile.toml");
    let second_config = "\
[Properties.other]
";

    // Act
    session.open_document(&second_uri, second_config);
    let error = session.read_notification("window/showMessage");
    let diagnostics = session.read_notification("textDocument/publishDiagnostics");
    let completion = session.completion(&file_uri, 2, 0, 17);

    // Assert
    assert!(
        error["params"]["message"]
            .as_str()
            .unwrap()
            .contains("conflicting config files")
    );
    assert!(
        diagnostics["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"].as_str().unwrap_or("").contains("config error"))
    );
    assert_eq!(completion["result"], serde_json::json!([]));
}

// A broken/conflicting mdagile.toml must not be silently swallowed by the
// LSP: unlike the CLI (which hard-fails with an error and a non-zero exit
// code), the server has to keep running, but it must still tell the user
// loudly that config-driven checks (E007-E013) are disabled — via both a
// `window/showMessage` notification and a synthetic diagnostic on the
// document being validated.

#[test]
fn lsp_reports_invalid_toml_via_show_message_and_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    let file_content = "\
this is not valid toml [[[
";
    std::fs::write(dir.path().join("mdagile.toml"), file_content).unwrap();

    let uri = file_uri(&dir.path().join("tasks.agile.md"));
    let root_uri = file_uri(dir.path());
    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(
        &uri,
        "\
- [ ] a task
",
    );

    let show_message = session.read_notification("window/showMessage");
    let text = show_message["params"]["message"].as_str().unwrap_or("");
    assert!(
        text.contains("config error"),
        "expected a config-error window/showMessage, got: {show_message:?}"
    );

    let diag_notification = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = diag_notification["params"]["diagnostics"]
        .as_array()
        .unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d["message"].as_str().unwrap_or("").contains("config error")),
        "expected a synthetic config-error diagnostic, but got: {diagnostics:?}"
    );
}

#[test]
fn lsp_reports_only_the_config_error_when_config_fails_to_load_not_spurious_undefined_marker_errors()
 {
    // With a broken mdagile.toml, the LSP must fall back to *no config
    // checks at all* — not to an empty Config, which would make every
    // #marker/@marker look "undefined" (E008/E009) even though the project's
    // real (unparseable) config might well have declared them.
    let dir = tempfile::tempdir().unwrap();
    let file_content = "\
this is not valid toml [[[
";
    std::fs::write(dir.path().join("mdagile.toml"), file_content).unwrap();

    let uri = file_uri(&dir.path().join("tasks.agile.md"));
    let root_uri = file_uri(dir.path());
    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(
        &uri,
        "\
- [ ] task #some_property @some_user
",
    );

    session.read_notification("window/showMessage");
    let diag_notification = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = diag_notification["params"]["diagnostics"]
        .as_array()
        .unwrap();

    assert!(
        !diagnostics
            .iter()
            .any(|d| d["code"].as_str() == Some("E008")),
        "expected no spurious E008 (undefined property) while config is broken, got: {diagnostics:?}"
    );
    assert!(
        !diagnostics
            .iter()
            .any(|d| d["code"].as_str() == Some("E009")),
        "expected no spurious E009 (undefined assignment) while config is broken, got: {diagnostics:?}"
    );
    // The only diagnostic should be the synthetic config-error one.
    assert_eq!(
        diagnostics.len(),
        1,
        "expected only the config-error diagnostic while config is broken, got: {diagnostics:?}"
    );
}

#[test]
fn lsp_reports_conflicting_config_files_via_show_message() {
    let dir = tempfile::tempdir().unwrap();
    // Both mdagile.toml and .mdagile.toml existing is an explicit
    // ConflictingConfig error in Config::load.
    let empty_config = "";
    std::fs::write(dir.path().join("mdagile.toml"), empty_config).unwrap();
    std::fs::write(dir.path().join(".mdagile.toml"), empty_config).unwrap();

    let uri = file_uri(&dir.path().join("tasks.agile.md"));
    let root_uri = file_uri(dir.path());
    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(
        &uri,
        "\
- [ ] a task
",
    );

    let show_message = session.read_notification("window/showMessage");
    let text = show_message["params"]["message"].as_str().unwrap_or("");
    assert!(
        text.contains("conflicting config files"),
        "expected a conflicting-config window/showMessage, got: {show_message:?}"
    );
}

#[test]
fn lsp_does_not_repeat_show_message_while_the_same_config_error_persists() {
    let dir = tempfile::tempdir().unwrap();
    let file_content = "\
this is not valid toml [[[
";
    std::fs::write(dir.path().join("mdagile.toml"), file_content).unwrap();

    let uri = file_uri(&dir.path().join("tasks.agile.md"));
    let root_uri = file_uri(dir.path());
    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    session.open_document(
        &uri,
        "\
- [ ] a task
",
    );
    // First validate() call: config is broken, error surfaces.
    session.read_notification("window/showMessage");
    session.read_notification("textDocument/publishDiagnostics");

    // Trigger a second validate() (a document edit) while the config is
    // still broken in exactly the same way.
    session.send(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": "- [ ] a task\n- [ ] another task\n" }]
            }
        })
        .to_string(),
    );

    // The only notification we should now see is the diagnostics republish;
    // no second window/showMessage should be queued ahead of it.
    loop {
        let msg = super::helpers::read_lsp_response(&mut session.reader)
            .expect("expected a message from server");
        let v: serde_json::Value = serde_json::from_str(&msg).expect("server sent invalid JSON");
        match v["method"].as_str() {
            Some("window/showMessage") => {
                panic!(
                    "unexpected repeated window/showMessage while config error is unchanged: {v:?}"
                )
            }
            Some("textDocument/publishDiagnostics") => break,
            _ => continue,
        }
    }
}
