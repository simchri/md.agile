use super::helpers::LspSession;

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
