use super::helpers::LspSession;

#[test]
fn lsp_e008_not_reported_for_declared_property() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("mdagile.toml"),
        "\
[Properties.priority]
",
    )
    .unwrap();

    let file_path = dir.path().join("tasks.agile.md");
    let uri = format!("file://{}", file_path.display());

    let mut session = LspSession::start();
    let doc_text = "\
- [ ] task #priority
";
    session.open_document(&uri, doc_text);

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
    let dir = tempfile::tempdir().unwrap();
    // No mdagile.toml — all properties are undeclared.
    let file_path = dir.path().join("tasks.agile.md");
    let uri = format!("file://{}", file_path.display());

    let mut session = LspSession::start();
    let doc_text = "\
- [ ] task #undeclared
";
    session.open_document(&uri, doc_text);

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
    let project_root = tempfile::tempdir().unwrap();
    std::fs::write(
        project_root.path().join("mdagile.toml"),
        "\
[Properties.priority]
",
    )
    .unwrap();

    let file_dir = tempfile::tempdir().unwrap();
    let file_path = file_dir.path().join("tasks.agile.md");
    let file_uri = format!("file://{}", file_path.display());
    let root_uri = format!("file://{}", project_root.path().display());

    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    let doc_text = "\
- [ ] task #priority
";
    session.open_document(&file_uri, doc_text);

    let notification = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = notification["params"]["diagnostics"].as_array().unwrap();

    assert!(
        !diagnostics
            .iter()
            .any(|d| d["code"].as_str() == Some("E008")),
        "expected no E008: server should use rootUri config, got: {diagnostics:?}"
    );
}
