use super::helpers::LspSession;

#[test]
fn lsp_goto_definition_resolves_assignment_to_config() {
    // GoTo Definition on `@alice` must jump to the `[Users.alice]` line in mdagile.toml.
    let project_root = tempfile::tempdir().unwrap();
    std::fs::write(
        project_root.path().join("mdagile.toml"),
        "\
[Users.alice]
display_name = \"Alice\"
",
    )
    .unwrap();

    let file_path = project_root.path().join("tasks.agile.md");
    let file_uri = format!("file://{}", file_path.display());
    let root_uri = format!("file://{}", project_root.path().display());
    let config_uri = format!(
        "file://{}",
        project_root.path().join("mdagile.toml").display()
    );

    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    let doc_text = "\
- [ ] task @alice
";
    session.open_document(&file_uri, doc_text);
    session.read_notification("textDocument/publishDiagnostics");

    let goto_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": file_uri },
            "position": { "line": 0, "character": 11 }
        }
    });
    session.send(&goto_req.to_string());

    let response = session.read_response(2);

    assert!(
        !response["result"].is_null(),
        "expected a location result, got: {response}"
    );
    assert_eq!(
        response["result"]["uri"].as_str().unwrap(),
        config_uri,
        "GoTo should point to mdagile.toml"
    );
    assert_eq!(
        response["result"]["range"]["start"]["line"], 0,
        "GoTo should point to line 0 ([Users.alice])"
    );
}

#[test]
fn lsp_goto_definition_resolves_group_assignment_to_config() {
    // GoTo Definition on `@backend` must jump to the `[Groups.backend]` line.
    let project_root = tempfile::tempdir().unwrap();
    std::fs::write(
        project_root.path().join("mdagile.toml"),
        "\
[Groups.backend]
",
    )
    .unwrap();

    let file_path = project_root.path().join("tasks.agile.md");
    let file_uri = format!("file://{}", file_path.display());
    let root_uri = format!("file://{}", project_root.path().display());

    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    let doc_text = "\
- [ ] task @backend
";
    session.open_document(&file_uri, doc_text);
    session.read_notification("textDocument/publishDiagnostics");

    let goto_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": file_uri },
            "position": { "line": 0, "character": 12 }
        }
    });
    session.send(&goto_req.to_string());

    let response = session.read_response(2);

    assert!(
        !response["result"].is_null(),
        "expected a location result, got: {response}"
    );
    assert_eq!(
        response["result"]["range"]["start"]["line"], 0,
        "GoTo should point to line 0 ([Groups.backend])"
    );
}

#[test]
fn lsp_goto_definition_returns_null_for_unknown_assignment() {
    // GoTo on `@nobody` when there is no matching entry in config → null result.
    let project_root = tempfile::tempdir().unwrap();
    std::fs::write(
        project_root.path().join("mdagile.toml"),
        "\
[Users.alice]
",
    )
    .unwrap();

    let file_path = project_root.path().join("tasks.agile.md");
    let file_uri = format!("file://{}", file_path.display());
    let root_uri = format!("file://{}", project_root.path().display());

    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    let doc_text = "\
- [ ] task @nobody
";
    session.open_document(&file_uri, doc_text);
    session.read_notification("textDocument/publishDiagnostics");

    let goto_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": file_uri },
            "position": { "line": 0, "character": 12 }
        }
    });
    session.send(&goto_req.to_string());

    let response = session.read_response(2);

    assert!(
        response["result"].is_null(),
        "expected null result for unknown assignment, got: {response}"
    );
}
