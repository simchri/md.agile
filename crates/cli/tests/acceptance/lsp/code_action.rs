use super::helpers::LspSession;

#[test]
fn lsp_typo_quickfix_preserves_trailing_punctuation() {
    // Arrange
    let config = "\
[Users.alice]
";
    let file_content = "\
- [ ] task @alce,
";
    let (mut session, file_uri) = super::helpers::start_project_session(config);
    session.open_document(&file_uri, file_content);
    let published = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = published["params"]["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|d| d["code"] == "E009"));

    // Act
    let request = serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/codeAction",
        "params": {
            "textDocument": {"uri": file_uri},
            "range": {"start": {"line": 0, "character": 0},
                      "end": {"line": 0, "character": 18}},
            "context": {"diagnostics": diagnostics}
        }
    });
    session.send(&request.to_string());
    let response = session.read_response(2);

    // Assert
    let actions = response["result"].as_array().unwrap();
    let correction = actions
        .iter()
        .find(|action| action["title"].as_str().unwrap_or("").contains("Fix typo"))
        .expect("correction should preserve comma");
    let edit = &correction["edit"]["changes"][&file_uri][0];
    assert_eq!(edit["newText"], "@alice");
    assert_eq!(edit["range"]["start"]["character"], 11);
    assert_eq!(edit["range"]["end"]["character"], 16);
}

#[test]
fn lsp_property_typo_quickfix_uses_utf16_range_for_unicode_marker() {
    // Arrange
    let config = "\
[Properties.feature]
";
    let file_content = "\
- [ ] 😀 #f😀ature
";
    let (mut session, file_uri) = super::helpers::start_project_session(config);
    session.open_document(&file_uri, file_content);
    let published = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = published["params"]["diagnostics"].as_array().unwrap();
    let undefined = diagnostics.iter().find(|d| d["code"] == "E008").unwrap();
    assert_eq!(undefined["range"]["end"]["character"], 9);

    // Act
    let request = serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/codeAction",
        "params": {
            "textDocument": {"uri": file_uri},
            "range": {"start": {"line": 0, "character": 0},
                      "end": {"line": 0, "character": 20}},
            "context": {"diagnostics": diagnostics}
        }
    });
    session.send(&request.to_string());
    let response = session.read_response(2);

    // Assert
    let actions = response["result"].as_array().unwrap();
    let correction = actions
        .iter()
        .find(|action| action["title"].as_str().unwrap_or("").contains("Fix typo"))
        .unwrap();
    let edit = &correction["edit"]["changes"][&file_uri][0];
    assert_eq!(edit["newText"], "#feature");
    assert_eq!(edit["range"]["start"]["character"], 9);
    assert_eq!(edit["range"]["end"]["character"], 18);
}

#[test]
fn lsp_typo_quickfix_uses_utf16_range_for_unicode_marker() {
    // Arrange
    let config = "\
[Users.alice]
";
    let file_content = "\
- [ ] 😀 @ali😀e
";
    let (mut session, file_uri) = super::helpers::start_project_session(config);
    session.open_document(&file_uri, file_content);
    let published = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = published["params"]["diagnostics"].as_array().unwrap();
    let undefined = diagnostics.iter().find(|d| d["code"] == "E009").unwrap();
    assert_eq!(undefined["range"]["end"]["character"], 9);

    // Act
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {"uri": file_uri},
            "range": {"start": {"line": 0, "character": 0},
                      "end": {"line": 0, "character": 17}},
            "context": {"diagnostics": diagnostics}
        }
    });
    session.send(&request.to_string());
    let response = session.read_response(2);

    // Assert
    let actions = response["result"].as_array().unwrap();
    let correction = actions
        .iter()
        .find(|action| action["title"].as_str().unwrap_or("").contains("Fix typo"))
        .unwrap();
    let edit = &correction["edit"]["changes"][&file_uri][0];
    assert_eq!(edit["newText"], "@alice");
    assert_eq!(edit["range"]["start"]["character"], 9);
    assert_eq!(edit["range"]["end"]["character"], 16);
}

#[test]
fn lsp_quickfix_uses_unsaved_config_without_overwriting_it() {
    // Arrange
    let config = "\
[Properties.feature]
";
    let file_content = "\
- [ ] task #featur
";
    let (mut session, file_uri) = super::helpers::start_project_session(config);
    let config_uri = file_uri.replace("tasks.agile.md", "mdagile.toml");
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");
    let unsaved_config = "\
[Properties.features]
brief = \"Unsaved notes\"
";
    session.open_document(&config_uri, unsaved_config);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let request = serde_json::json!({
        "jsonrpc":"2.0", "id":2, "method":"textDocument/codeAction",
        "params":{
            "textDocument":{"uri":file_uri},
            "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":19}},
            "context":{"diagnostics":[]}
        }
    });
    session.send(&request.to_string());
    let response = session.read_response(2);

    // Assert
    let actions = response["result"].as_array().expect("actions");
    assert!(
        actions
            .iter()
            .any(|action| action["title"].as_str().unwrap_or("").contains("#features"))
    );
    assert!(
        !actions
            .iter()
            .any(|action| action["title"].as_str().unwrap_or("").contains("#feature'"))
    );
    let add = actions
        .iter()
        .find(|action| {
            action["title"]
                .as_str()
                .unwrap_or("")
                .contains("Add '[Properties.featur]'")
        })
        .expect("add action");
    let edit = &add["edit"]["changes"][&config_uri][0]["newText"];
    assert!(edit.as_str().unwrap().contains("Unsaved notes"));
}

#[test]
fn lsp_code_action_returns_quickfix_for_e002() {
    let mut session = LspSession::start();

    // Open a document with a wrong-indentation (E002) issue.
    // 3-space indent on the subtask — correct is 2.
    let uri = "file:///tmp/test_quickfix.agile.md";
    let doc_text = "\
- [ ] top
   - [ ] sub
";
    session.open_document(uri, doc_text);

    // Collect the diagnostics the server published so we can pass them back
    // in the codeAction request context, as a real editor would.
    let diag_notification = session.read_notification("textDocument/publishDiagnostics");
    let diagnostics = diag_notification["params"]["diagnostics"].clone();
    assert!(
        diagnostics.as_array().map_or(false, |a| !a.is_empty()),
        "expected at least one diagnostic from the server"
    );

    let code_action_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 1, "character": 0 },
                "end":   { "line": 1, "character": 3 }
            },
            "context": {
                "diagnostics": diagnostics,
                "triggerKind": 1
            }
        }
    });
    session.send(&code_action_request.to_string());

    let response = session.read_response(2);

    let actions = response["result"]
        .as_array()
        .expect("result should be an array");
    assert!(!actions.is_empty(), "expected at least one code action");
    assert!(
        actions
            .iter()
            .any(|a| a["kind"].as_str() == Some("quickfix")),
        "expected a quickfix action, got: {response}"
    );
}

#[test]
fn lsp_code_action_works_when_client_strips_data_field() {
    // Neovim does not round-trip the `data` field from publishDiagnostics back
    // in codeAction context.diagnostics. The server must not rely on it.
    let mut session = LspSession::start();

    let uri = "file:///tmp/test_quickfix_no_data.agile.md";
    let doc_text = "\
- [ ] top
   - [ ] sub
";
    session.open_document(uri, doc_text);

    let diag_notification = session.read_notification("textDocument/publishDiagnostics");
    let mut diagnostics = diag_notification["params"]["diagnostics"].clone();

    // Strip `data` from every diagnostic — simulating what Neovim does.
    for d in diagnostics.as_array_mut().unwrap() {
        d.as_object_mut().unwrap().remove("data");
    }

    let code_action_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 1, "character": 0 },
                "end":   { "line": 1, "character": 3 }
            },
            "context": {
                "diagnostics": diagnostics,
                "triggerKind": 1
            }
        }
    });
    session.send(&code_action_request.to_string());

    let response = session.read_response(2);

    let actions = response["result"]
        .as_array()
        .expect("result should be an array");
    assert!(!actions.is_empty(), "expected at least one code action");
    assert!(
        actions
            .iter()
            .any(|a| a["kind"].as_str() == Some("quickfix")),
        "expected a quickfix action, got: {response}"
    );
}

#[test]
fn lsp_code_action_available_anywhere_on_the_line() {
    // The quickfix for E002 should be offered regardless of where the cursor
    // sits on the offending line, not only when it is in the leading whitespace.
    let mut session = LspSession::start();

    let uri = "file:///tmp/test_quickfix_cursor.agile.md";
    let doc_text = "\
- [ ] top
   - [ ] sub
";
    session.open_document(uri, doc_text);
    session.read_notification("textDocument/publishDiagnostics");

    // Cursor is at the end of the line, well past the 3-space indent region.
    let code_action_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 1, "character": 14 },
                "end":   { "line": 1, "character": 14 }
            },
            "context": { "diagnostics": [], "triggerKind": 1 }
        }
    });
    session.send(&code_action_request.to_string());

    let response = session.read_response(2);

    let actions = response["result"]
        .as_array()
        .expect("result should be an array");
    assert!(
        actions
            .iter()
            .any(|a| a["kind"].as_str() == Some("quickfix")),
        "expected a quickfix action, got: {response}"
    );
}
