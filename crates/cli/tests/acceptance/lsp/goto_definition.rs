use super::helpers::{file_uri, start_project_session};

#[test]
fn lsp_goto_definition_resolves_assignment_to_config() {
    // GoTo Definition on `@alice` must jump to the `[Users.alice]` line in mdagile.toml.
    let (mut session, file_uri) = start_project_session(
        "\
[Users.alice]
display_name = \"Alice\"
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
