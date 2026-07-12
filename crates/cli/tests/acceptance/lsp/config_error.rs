use super::helpers::{LspSession, file_uri};

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
