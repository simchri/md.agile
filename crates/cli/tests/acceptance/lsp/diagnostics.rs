use super::helpers::{LspSession, file_uri, start_project_session};

#[test]
fn lsp_e008_not_reported_for_declared_property() {
    let dir = tempfile::tempdir().unwrap();
    let file_content = "\
[Properties.priority]
";
    std::fs::write(dir.path().join("mdagile.toml"), file_content).unwrap();

    let uri = file_uri(&dir.path().join("tasks.agile.md"));
    let mut session = LspSession::start();
    session.open_document(
        &uri,
        "\
- [ ] task #priority
",
    );

    let notification = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = notification["params"]["diagnostics"].as_array().unwrap();

    assert!(
        !diagnostics
            .iter()
            .any(|d| d["code"].as_str() == Some("E008")),
        "expected no E008 for declared property '#priority', but got: {diagnostics:?}"
    );
}

#[test]
fn lsp_e008_reported_for_undeclared_property() {
    // No mdagile.toml — all properties are undeclared.
    let dir = tempfile::tempdir().unwrap();
    let uri = file_uri(&dir.path().join("tasks.agile.md"));

    let mut session = LspSession::start();
    session.open_document(
        &uri,
        "\
- [ ] task #undeclared
",
    );

    let notification = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = notification["params"]["diagnostics"].as_array().unwrap();

    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"].as_str() == Some("E008")),
        "expected E008 for undeclared property '#undeclared', but got: {diagnostics:?}"
    );
}

#[test]
fn lsp_uses_root_uri_for_config_not_file_walk() {
    // mdagile.toml lives in the project root (rootUri dir).
    // The .agile.md file lives in a completely separate temp dir.
    // Walk-up from the file dir will never reach the project root,
    // so only a rootUri-aware server will avoid a false E008.
    let (mut session, _) = start_project_session(
        "\
[Properties.priority]
",
    );
    // Use a *different* tempdir for the file — walk-up will never find the config.
    let file_dir = tempfile::tempdir().unwrap();
    let file_uri = file_uri(&file_dir.path().join("tasks.agile.md"));

    session.open_document(
        &file_uri,
        "\
- [ ] task #priority
",
    );

    let notification = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = notification["params"]["diagnostics"].as_array().unwrap();

    assert!(
        !diagnostics
            .iter()
            .any(|d| d["code"].as_str() == Some("E008")),
        "expected no E008: server should use rootUri config, got: {diagnostics:?}"
    );
}
