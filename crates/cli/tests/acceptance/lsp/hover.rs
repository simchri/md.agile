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

    let contents = response["result"]["contents"]["value"]
        .as_str()
        .expect("expected Markdown hover contents");
    assert_eq!(
        contents,
        "\
**#feature**

New end-user visible functionality

A customer-facing change.

**Required subtasks:**

- design
- implementation"
    );
    assert!(!contents.contains("Brief:"));
    assert!(!contents.contains("Description:"));
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

#[test]
fn lsp_hover_explains_user_assignment() {
    // Arrange
    let config = "\
[Users.alice]
";
    let file_content = "\
- [ ] task @alice
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.hover(&file_uri, 2, 0, 14);

    // Assert
    let contents = response["result"]["contents"]["value"]
        .as_str()
        .expect("expected assignment hover contents");

    assert!(contents.contains("@alice"), "hover: {contents}");
    assert!(contents.contains("user"), "hover: {contents}");
    assert!(
        contents.contains("eligible to work on this task"),
        "hover: {contents}"
    );
    assert!(
        contents.contains("may mark it complete"),
        "hover: {contents}"
    );
}

#[test]
fn lsp_hover_shows_group_assignment_members() {
    // Arrange
    let config = "\
[Users.alice]

[Users.bob]

[Groups.devs]
members = [\"alice\", \"bob\"]
";
    let file_content = "\
- [ ] task @devs
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.hover(&file_uri, 2, 0, 13);

    // Assert
    let contents = response["result"]["contents"]["value"]
        .as_str()
        .expect("expected assignment hover contents");

    assert!(contents.contains("@devs"), "hover: {contents}");
    assert!(contents.contains("group"), "hover: {contents}");
    assert!(contents.contains("alice"), "hover: {contents}");
    assert!(contents.contains("bob"), "hover: {contents}");
    assert!(
        contents.contains("eligible to work on this task"),
        "hover: {contents}"
    );
    assert!(
        contents.contains("may mark it complete"),
        "hover: {contents}"
    );
}

#[test]
fn lsp_hover_returns_null_for_unknown_assignment() {
    // Arrange
    let config = "";
    let file_content = "\
- [ ] task @unknown
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.hover(&file_uri, 2, 0, 14);

    // Assert
    assert!(
        response["result"].is_null(),
        "expected no hover for unknown assignment, got: {response}"
    );
}

#[test]
fn lsp_hover_explains_assignment_matching_user_and_group() {
    // Arrange
    let config = "\
[Users.alice]

[Groups.alice]
members = [\"bob\"]

[Users.bob]
";
    let file_content = "\
- [ ] task @alice
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.hover(&file_uri, 2, 0, 14);

    // Assert
    let contents = response["result"]["contents"]["value"]
        .as_str()
        .expect("expected assignment hover contents");
    assert!(contents.contains("both user `alice` and group `alice`"));
    assert!(contents.contains("`bob`"));
}

#[test]
fn lsp_hover_explains_optional_subtask_marker() {
    // Arrange
    let config = "";
    let file_content = "\
- [ ] parent
  - [ ] #OPT optional polish
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.hover(&file_uri, 2, 1, 10);

    // Assert
    let contents = response["result"]["contents"]["value"]
        .as_str()
        .expect("expected optional-subtask hover contents");
    assert_eq!(
        contents,
        "**#OPT**\n\nOptional subtask. This subtask does not block completion of its parent task."
    );
}

#[test]
fn lsp_hover_explains_milestone_marker() {
    // Arrange
    let config = "";
    let file_content = "\
- [x] finish release
#MILESTONE: Release
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.hover(&file_uri, 2, 1, 4);

    // Assert
    let contents = response["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("expected milestone hover contents: {response}"));
    assert_eq!(
        contents,
        "**#MILESTONE**\n\nA milestone separates tasks in the backlog. It is reached when all tasks before it are complete."
    );
}

#[test]
fn lsp_hover_explains_md_agile_marker_without_claiming_unimplemented_directive() {
    // Arrange
    let config = "";
    let file_content = "\
#MDAGILE.file.mandatory_property=feature
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.hover(&file_uri, 2, 0, 3);

    // Assert
    let contents = response["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("expected MDAGILE hover contents: {response}"));
    assert_eq!(
        contents,
        "**#MDAGILE**\n\nReserved for file-level Mdagile directives. The `file.mandatory_property` directive is not currently implemented."
    );
}

#[test]
fn lsp_hover_only_explains_special_markers_in_their_syntax_positions() {
    // Arrange
    let config = "";
    let file_content = "\
- [ ] #MILESTONE: not a standalone milestone
#MDAGILE.file.mandatory_property=feature
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let inline_milestone = session.hover(&file_uri, 2, 0, 8);
    let directive_value = session.hover(&file_uri, 3, 1, 15);

    // Assert
    assert!(
        inline_milestone["result"].is_null(),
        "expected no hover for an inline #MILESTONE, got: {inline_milestone}"
    );
    assert!(
        directive_value["result"].is_null(),
        "expected no hover over the MDAGILE directive value, got: {directive_value}"
    );
}
