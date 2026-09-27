use super::helpers::{LspSession, read_lsp_response, send_lsp_message, start_project_session};

#[test]
fn lsp_completion_suggests_matching_properties_and_replaces_partial_marker() {
    // Arrange
    let config = "\
[Properties.feature]
brief = \"Feature brief\"
description = \"More about the feature.\"
subtasks = [\"design\", \"implementation\"]

[Properties.feat]

[Properties.bug]
";
    let file_content = "\
- [ ] task #fe
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.completion(&file_uri, 2, 0, 14);

    // Assert
    let items = response["result"]
        .as_array()
        .expect("expected completion item array");
    let labels: Vec<_> = items
        .iter()
        .filter_map(|item| item["label"].as_str())
        .collect();
    assert_eq!(labels, vec!["#feat", "#feature"]);
    for item in items {
        assert_eq!(item["kind"], 10);
        assert_eq!(item["textEdit"]["newText"], item["label"]);
        assert_eq!(item["textEdit"]["range"]["start"]["character"], 11);
        assert_eq!(item["textEdit"]["range"]["end"]["character"], 14);
    }

    assert_eq!(items[0]["documentation"], serde_json::Value::Null);
    let expected_documentation = "\
**#feature**

Feature brief

More about the feature.

**Required subtasks:**

- design
- implementation";
    assert_eq!(
        items[1]["documentation"],
        serde_json::json!({"kind": "markdown", "value": expected_documentation})
    );
}

#[test]
fn lsp_completion_uses_utf16_replacement_range_after_emoji() {
    // Arrange
    let config = "\
[Properties.feature]
";
    let file_content = "\
- [ ] 😀 #fe
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.completion(&file_uri, 2, 0, 12);

    // Assert
    let item = &response["result"][0];
    assert_eq!(item["label"], "#feature");
    assert_eq!(item["textEdit"]["range"]["start"]["character"], 9);
    assert_eq!(item["textEdit"]["range"]["end"]["character"], 12);
}

#[test]
fn lsp_completion_reads_new_unsaved_config_file() {
    // Arrange
    let dir = tempfile::tempdir().unwrap();
    let root_uri = super::helpers::file_uri(dir.path());
    let file_uri = super::helpers::file_uri(&dir.path().join("tasks.agile.md"));
    let config_uri = super::helpers::file_uri(&dir.path().join("mdagile.toml"));
    let mut session = LspSession::start_with_root_uri(Some(&root_uri));
    let file_content = "\
- [ ] task #fe
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");
    let config = "\
[Properties.feature]
";
    session.open_document(&config_uri, config);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.completion(&file_uri, 2, 0, 14);

    // Assert
    assert_eq!(response["result"][0]["label"], "#feature");
}

#[test]
fn lsp_completion_suggests_all_properties_immediately_after_hash() {
    // Arrange
    let config = "\
[Properties.feature]

[Properties.bug]
";
    let file_content = "\
- [ ] task #
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.completion(&file_uri, 2, 0, 12);

    // Assert
    let items = response["result"]
        .as_array()
        .expect("expected completion item array");
    let labels: Vec<_> = items
        .iter()
        .filter_map(|item| item["label"].as_str())
        .collect();
    assert_eq!(labels, vec!["#bug", "#feature"]);
}

#[test]
fn lsp_completion_returns_empty_outside_property_marker() {
    // Arrange
    let config = "\
[Properties.feature]
";
    let file_content = "\
- [ ] task feature
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.completion(&file_uri, 2, 0, 14);

    // Assert
    assert_eq!(response["result"], serde_json::json!([]));
}

#[test]
fn lsp_initialize_advertises_marker_completion_triggers() {
    // Arrange
    let mut session = LspSession::start_raw();
    send_lsp_message(
        &mut session.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#,
    )
    .unwrap();

    // Act
    let response = read_lsp_response(&mut session.reader).unwrap();
    let value: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Assert
    assert_eq!(
        value["result"]["capabilities"]["completionProvider"]["triggerCharacters"],
        serde_json::json!(["#", "@"]),
        "response: {response}"
    );
}

#[test]
fn lsp_completion_suggests_users_and_groups_with_hover_documentation() {
    // Arrange
    let config = "\
[Users.alice]

[Users.amy]

[Groups.admins]
members = [\"alice\", \"amy\"]

[Groups.devs]
members = [\"amy\"]
";
    let file_content = "\
- [ ] task @a
- [ ] task @admins
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.completion(&file_uri, 2, 0, 13);

    // Assert
    let items = response["result"].as_array().expect("completion items");
    let labels: Vec<_> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap())
        .collect();
    assert_eq!(labels, vec!["@admins", "@alice", "@amy"]);
    assert_eq!(items[0]["kind"], 9);
    assert_eq!(items[1]["kind"], 6);
    for item in items {
        assert_eq!(item["textEdit"]["newText"], item["label"]);
        assert_eq!(item["textEdit"]["range"]["start"]["character"], 11);
        assert_eq!(item["textEdit"]["range"]["end"]["character"], 13);
        assert_eq!(item["documentation"]["kind"], "markdown");
    }
    let group_hover = session.hover(&file_uri, 3, 1, 12);
    assert_eq!(items[0]["documentation"], group_hover["result"]["contents"]);
    assert_eq!(
        items[0]["documentation"]["value"],
        "**@admins**\n\nAssigned to group `admins`.\n\n**Members:**\n\n- `alice`\n- `amy`\n\nAssignments determine who is eligible to work on this task and who may mark it complete."
    );
    assert_eq!(
        items[1]["documentation"]["value"],
        "**@alice**\n\nAssigned to user `alice`.\n\nAssignments determine who is eligible to work on this task and who may mark it complete."
    );
}

#[test]
fn lsp_completion_shows_group_metadata_from_unsaved_config() {
    // Arrange
    let config = "\
[Users.alice]

[Groups.devs]
members = [\"alice\"]
";
    let file_content = "\
- [ ] task @de
- [ ] task @devs
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");
    let config_uri = file_uri.replace("tasks.agile.md", "mdagile.toml");
    let unsaved_config = "\
[Users.alice]

[Groups.devs]
brief = \"Product development team\"
description = \"Builds and maintains the product.\"
members = [\"alice\"]
";
    session.open_document(&config_uri, unsaved_config);

    // Act
    let completion = session.completion(&file_uri, 2, 0, 14);
    let hover = session.hover(&file_uri, 3, 1, 13);

    // Assert
    let items = completion["result"].as_array().expect("completion items");
    assert_eq!(items.len(), 1);
    let expected = "\
**@devs**

Assigned to group `devs`.

Product development team

Builds and maintains the product.

**Members:**

- `alice`

Assignments determine who is eligible to work on this task and who may mark it complete.";
    assert_eq!(items[0]["documentation"]["kind"], "markdown");
    assert_eq!(items[0]["documentation"]["value"], expected);
    assert_eq!(items[0]["documentation"], hover["result"]["contents"]);
}

#[test]
fn lsp_completion_after_at_suggests_all_assignments_and_replaces_full_marker() {
    // Arrange
    let config = "\
[Users.bob]

[Groups.devs]
members = [\"bob\"]
";
    let file_content = "\
- [ ] task @devs
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.completion(&file_uri, 2, 0, 12);

    // Assert
    let items = response["result"].as_array().expect("completion items");
    let labels: Vec<_> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap())
        .collect();
    assert_eq!(labels, vec!["@bob", "@devs"]);
    for item in items {
        assert_eq!(item["textEdit"]["range"]["start"]["character"], 11);
        assert_eq!(item["textEdit"]["range"]["end"]["character"], 16);
    }
}

#[test]
fn lsp_completion_deduplicates_user_and_group_with_same_name() {
    // Arrange
    let config = "\
[Users.alice]

[Groups.alice]
members = [\"alice\"]
";
    let file_content = "\
- [ ] task @al
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.completion(&file_uri, 2, 0, 14);

    // Assert
    let items = response["result"].as_array().expect("completion items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["label"], "@alice");
    assert!(
        items[0]["documentation"]["value"]
            .as_str()
            .unwrap()
            .contains("both user `alice` and group `alice`")
    );
}

#[test]
fn lsp_completion_does_not_suggest_special_markers_or_code_span_content() {
    // Arrange
    let config = "\
[Properties.feature]
";
    let file_content = "\
- [ ] task #OPT
- [ ] task `#fe`
";
    let (mut session, file_uri) = start_project_session(config);
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let special_marker = session.completion(&file_uri, 2, 0, 14);
    let start_of_special_marker = session.completion(&file_uri, 3, 0, 12);
    let code_span = session.completion(&file_uri, 3, 0, 15);

    // Assert
    assert_eq!(special_marker["result"], serde_json::json!([]));
    assert_eq!(start_of_special_marker["result"], serde_json::json!([]));
    assert_eq!(code_span["result"], serde_json::json!([]));
}
