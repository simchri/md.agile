use super::helpers::{LspSession, read_lsp_response, send_lsp_message, start_project_session};

#[test]
fn lsp_hover_shows_property_brief_description_and_required_subtasks() {
    let (mut session, file_uri) = start_project_session(
        "\
[Properties.feature]
brief = \"New end-user visible functionality\"
description = \"A customer-facing change.\"
subtasks = [\"design\", \"implementation\"]
",
    );
    session.open_document(
        &file_uri,
        "\
- [ ] #feature: add basket
",
    );
    session.read_notification("textDocument/publishDiagnostics");

    let response = session.hover(&file_uri, 2, 0, 10);

    assert!(
        response["result"]["contents"]["value"]
            .as_str()
            .is_some_and(|contents| {
                contents.contains("New end-user visible functionality")
                    && contents.contains("A customer-facing change.")
                    && contents.contains("design")
                    && contents.contains("implementation")
            }),
        "expected property details in hover response, got: {response}"
    );
}

#[test]
fn lsp_initialize_advertises_hover_provider() {
    let mut session = LspSession::start_raw();
    send_lsp_message(
        &mut session.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#,
    )
    .unwrap();

    let response = read_lsp_response(&mut session.reader).unwrap();
    let value: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(
        value["result"]["capabilities"]["hoverProvider"],
        serde_json::json!(true),
        "response: {response}"
    );
}

#[test]
fn lsp_hover_returns_null_outside_property_markers() {
    let (mut session, file_uri) = start_project_session(
        "\
[Properties.feature]
brief = \"Feature summary\"
",
    );
    session.open_document(
        &file_uri,
        "\
- [ ] #feature: add basket
",
    );
    session.read_notification("textDocument/publishDiagnostics");

    let response = session.hover(&file_uri, 2, 0, 2);

    assert!(
        response["result"].is_null(),
        "expected no hover outside the marker, got: {response}"
    );
}

#[test]
fn lsp_hover_uses_unsaved_property_config() {
    let (mut session, file_uri) = start_project_session(
        "\
[Properties.feature]
brief = \"Saved summary\"
",
    );
    let config_uri = file_uri.replace("tasks.agile.md", "mdagile.toml");
    session.open_document(
        &file_uri,
        "\
- [ ] #feature: add basket
",
    );
    session.read_notification("textDocument/publishDiagnostics");
    session.open_document(
        &config_uri,
        "\
[Properties.feature]
brief = \"Unsaved summary\"
subtasks = [\"review\"]
",
    );

    let response = session.hover(&file_uri, 2, 0, 10);
    let contents = response["result"]["contents"]["value"].as_str().unwrap();

    assert!(contents.contains("Unsaved summary"), "hover: {contents}");
    assert!(contents.contains("review"), "hover: {contents}");
    assert!(!contents.contains("Saved summary"), "hover: {contents}");
}

#[test]
fn lsp_hover_returns_null_when_property_has_no_hover_details() {
    let (mut session, file_uri) = start_project_session(
        "\
[Properties.feature]
",
    );
    session.open_document(
        &file_uri,
        "\
- [ ] #feature: add basket
",
    );
    session.read_notification("textDocument/publishDiagnostics");

    let response = session.hover(&file_uri, 2, 0, 8);

    assert!(
        response["result"].is_null(),
        "expected no hover for property without details, got: {response}"
    );
}
