use super::helpers::start_project_session;

#[test]
fn lsp_definition_resolves_marker_after_utf16_surrogate_pair() {
    // Arrange
    let config = "\
[Properties.feature]
";
    let file_content = "\
- [ ] 😀 #feature
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.goto_definition(&file_uri, 2, 0, 16);

    // Assert
    assert_eq!(response["result"]["range"]["start"]["line"], 0);
}

#[test]
fn lsp_definition_follows_unsaved_config_and_closed_buffer() {
    // Arrange
    let config = "\
[Properties.old]
";
    let file_content = "\
- [ ] task #new
";
    let (mut session, file_uri) = start_project_session(config);
    let config_uri = file_uri.replace("tasks.agile.md", "mdagile.toml");
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");
    let unsaved_config = "\
[Properties.new]
";
    session.open_document(&config_uri, unsaved_config);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let before_close = session.goto_definition(&file_uri, 2, 0, 12);
    session.send(
        &serde_json::json!({
            "jsonrpc":"2.0", "method":"textDocument/didClose",
            "params":{"textDocument":{"uri":config_uri}}
        })
        .to_string(),
    );
    session.read_notification("textDocument/publishDiagnostics");
    session.read_notification("textDocument/publishDiagnostics");
    let after_close = session.goto_definition(&file_uri, 3, 0, 12);

    // Assert
    assert_eq!(before_close["result"]["range"]["start"]["line"], 0);
    assert!(after_close["result"].is_null());
}

#[test]
fn lsp_goto_definition_resolves_assignment_to_config() {
    // GoTo Definition on `@alice` must jump to the `[Users.alice]` line in mdagile.toml.
    let (mut session, file_uri) = start_project_session(
        "\
[Users.alice]
git_names = [\"Alice\"]
",
    );
    let config_uri = file_uri.replace("tasks.agile.md", "mdagile.toml");

    session.open_document(
        &file_uri,
        "\
- [ ] task @alice
",
    );
    session.read_notification("textDocument/publishDiagnostics");

    let response = session.goto_definition(&file_uri, 2, 0, 11);

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
    let (mut session, file_uri) = start_project_session(
        "\
[Groups.backend]
",
    );
    session.open_document(
        &file_uri,
        "\
- [ ] task @backend
",
    );
    session.read_notification("textDocument/publishDiagnostics");

    let response = session.goto_definition(&file_uri, 2, 0, 12);

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
    let (mut session, file_uri) = start_project_session(
        "\
[Users.alice]
",
    );
    session.open_document(
        &file_uri,
        "\
- [ ] task @nobody
",
    );
    session.read_notification("textDocument/publishDiagnostics");

    let response = session.goto_definition(&file_uri, 2, 0, 12);

    assert!(
        response["result"].is_null(),
        "expected null result for unknown assignment, got: {response}"
    );
}
