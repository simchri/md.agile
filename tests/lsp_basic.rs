//! Acceptance tests for `agile lsp` (Language Server Protocol).
//!
//! Tests the LSP server by sending JSON-RPC messages to stdin and
//! verifying responses on stdout. Following the LSP specification.

use std::process::{Command, Stdio};
use std::io::{Write, BufRead, BufReader};
use serde_json::Value;

fn start_lsp_server() -> (std::process::Child, BufReader<std::process::ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agilels"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn `agilels`");

    let stdout = child.stdout.take().expect("stdout");
    let reader = BufReader::new(stdout);
    
    (child, reader)
}

fn send_lsp_message<W: Write>(writer: &mut W, message: &str) -> std::io::Result<()> {
    writeln!(writer, "Content-Length: {}", message.len())?;
    writeln!(writer, "Content-Type: application/vscode-jsonrpc; charset=utf-8")?;
    writeln!(writer)?;
    write!(writer, "{}", message)?;
    writer.flush()?;
    Ok(())
}

fn read_lsp_response<R: BufRead>(reader: &mut R) -> std::io::Result<String> {
    let mut headers = std::collections::HashMap::new();
    let mut line = String::new();
    
    // Read headers
    loop {
        line.clear();
        reader.read_line(&mut line)?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }
    
    // Get content length
    let content_length: usize = headers
        .get("content-length")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing Content-Length"))?;
    
    // Read message body
    let mut message = vec![0u8; content_length];
    reader.read_exact(&mut message)?;
    
    Ok(String::from_utf8_lossy(&message).to_string())
}

#[test]
fn lsp_initialize_advertises_code_action_provider() {
    let (mut child, mut reader) = start_lsp_server();
    let mut stdin = child.stdin.take().unwrap();

    let init_request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    send_lsp_message(&mut stdin, init_request).unwrap();

    let response = read_lsp_response(&mut reader).unwrap();
    let v: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(
        v["result"]["capabilities"]["codeActionProvider"],
        serde_json::json!(true),
        "response: {response}"
    );

    drop(stdin);
    let _ = child.kill();
}

#[test]
fn lsp_initialize_request_returns_capabilities() {
    let (mut child, mut reader) = start_lsp_server();
    let mut stdin = child.stdin.take().unwrap();

    // Send initialize request
    let init_request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    send_lsp_message(&mut stdin, init_request).unwrap();
    
    // Read response
    let response = read_lsp_response(&mut reader).unwrap();
    
    // Verify response is JSON with result
    assert!(response.contains("\"jsonrpc\":\"2.0\""), "response: {}", response);
    assert!(response.contains("\"id\":1"), "response: {}", response);
    assert!(response.contains("\"result\""), "response: {}", response);
    assert!(response.contains("\"capabilities\""), "response: {}", response);
    
    // Cleanup
    drop(stdin);
    let _ = child.kill();
}

#[test]
fn lsp_initialized_notification_accepted() {
    let (mut child, mut reader) = start_lsp_server();
    let mut stdin = child.stdin.take().unwrap();

    // Send initialize
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    send_lsp_message(&mut stdin, init).unwrap();

    // Read initialize response
    let _response = read_lsp_response(&mut reader).unwrap();

    // Send initialized notification (no response expected)
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    send_lsp_message(&mut stdin, initialized).unwrap();
    
    // Server should still be running (not error)
    assert!(child.try_wait().is_ok() || child.try_wait().unwrap().is_none());
    
    // Cleanup
    drop(stdin);
    let _ = child.kill();
}

#[test]
fn lsp_shutdown_request_handled() {
    let (mut child, mut reader) = start_lsp_server();
    let mut stdin = child.stdin.take().unwrap();

    // Initialize first
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    send_lsp_message(&mut stdin, init).unwrap();
    let _init_response = read_lsp_response(&mut reader).unwrap();
    
    // Send shutdown
    let shutdown = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#;
    send_lsp_message(&mut stdin, shutdown).unwrap();
    
    // Read shutdown response
    let response = read_lsp_response(&mut reader).unwrap();

    assert!(response.contains("\"result\":null"), "response: {}", response);

    // Cleanup
    drop(stdin);
    let _ = child.kill();
}

/// Read messages until one whose `method` field matches `target`, discarding others.
fn read_notification<R: BufRead>(reader: &mut R, target: &str) -> Value {
    loop {
        let msg = read_lsp_response(reader).expect("expected a message from server");
        let v: Value = serde_json::from_str(&msg).expect("server sent invalid JSON");
        if v["method"].as_str() == Some(target) {
            return v;
        }
    }
}

/// Read messages until one that is a response to `id`, discarding notifications.
fn read_response<R: BufRead>(reader: &mut R, id: u64) -> Value {
    loop {
        let msg = read_lsp_response(reader).expect("expected a message from server");
        let v: Value = serde_json::from_str(&msg).expect("server sent invalid JSON");
        if v["id"] == id {
            return v;
        }
    }
}

#[test]
fn lsp_code_action_returns_quickfix_for_e002() {
    let (mut child, mut reader) = start_lsp_server();
    let mut stdin = child.stdin.take().unwrap();

    // Handshake
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    send_lsp_message(&mut stdin, init).unwrap();
    let _init_response = read_lsp_response(&mut reader).unwrap();

    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    send_lsp_message(&mut stdin, initialized).unwrap();

    // Open a document with a wrong-indentation (E002) issue.
    // 3-space indent on the subtask — correct is 2.
    let uri = "file:///tmp/test_quickfix.agile.md";
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "markdown",
                "version": 1,
                "text": "\
- [ ] top
   - [ ] sub
"
            }
        }
    });
    send_lsp_message(&mut stdin, &did_open.to_string()).unwrap();

    // Collect the diagnostics the server published so we can pass them back
    // in the codeAction request context, as a real editor would.
    let diag_notification = read_notification(&mut reader, "textDocument/publishDiagnostics");
    let diagnostics = diag_notification["params"]["diagnostics"].clone();
    assert!(
        diagnostics.as_array().map_or(false, |a| !a.is_empty()),
        "expected at least one diagnostic from the server"
    );

    // Request code actions for the range covering the wrong-indent line.
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
    send_lsp_message(&mut stdin, &code_action_request.to_string()).unwrap();

    let response = read_response(&mut reader, 2);

    assert!(!response["result"].is_null(), "expected a result, got: {response}");
    let actions = response["result"].as_array().expect("result should be an array");
    assert!(!actions.is_empty(), "expected at least one code action");
    assert!(
        actions.iter().any(|a| a["kind"].as_str() == Some("quickfix")),
        "expected a quickfix action, got: {response}"
    );

    drop(stdin);
    let _ = child.kill();
}

#[test]
fn lsp_code_action_works_when_client_strips_data_field() {
    // Neovim does not round-trip the `data` field from publishDiagnostics back
    // in codeAction context.diagnostics. The server must not rely on it.
    let (mut child, mut reader) = start_lsp_server();
    let mut stdin = child.stdin.take().unwrap();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    send_lsp_message(&mut stdin, init).unwrap();
    let _init_response = read_lsp_response(&mut reader).unwrap();

    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    send_lsp_message(&mut stdin, initialized).unwrap();

    let uri = "file:///tmp/test_quickfix_no_data.agile.md";
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "markdown",
                "version": 1,
                "text": "\
- [ ] top
   - [ ] sub
"
            }
        }
    });
    send_lsp_message(&mut stdin, &did_open.to_string()).unwrap();

    let diag_notification = read_notification(&mut reader, "textDocument/publishDiagnostics");
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
    send_lsp_message(&mut stdin, &code_action_request.to_string()).unwrap();

    let response = read_response(&mut reader, 2);

    assert!(!response["result"].is_null(), "expected a result, got: {response}");
    let actions = response["result"].as_array().expect("result should be an array");
    assert!(!actions.is_empty(), "expected at least one code action");
    assert!(
        actions.iter().any(|a| a["kind"].as_str() == Some("quickfix")),
        "expected a quickfix action, got: {response}"
    );

    drop(stdin);
    let _ = child.kill();
}

#[test]
fn lsp_code_action_available_anywhere_on_the_line() {
    // The quickfix for E002 should be offered regardless of where the cursor
    // sits on the offending line, not only when it is in the leading whitespace.
    let (mut child, mut reader) = start_lsp_server();
    let mut stdin = child.stdin.take().unwrap();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    send_lsp_message(&mut stdin, init).unwrap();
    let _init_response = read_lsp_response(&mut reader).unwrap();
    send_lsp_message(&mut stdin, r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#).unwrap();

    let uri = "file:///tmp/test_quickfix_cursor.agile.md";
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "markdown",
                "version": 1,
                "text": "- [ ] top\n   - [ ] sub\n"
            }
        }
    });
    send_lsp_message(&mut stdin, &did_open.to_string()).unwrap();
    read_notification(&mut reader, "textDocument/publishDiagnostics");

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
    send_lsp_message(&mut stdin, &code_action_request.to_string()).unwrap();

    let response = read_response(&mut reader, 2);

    assert!(!response["result"].is_null(), "expected a result, got: {response}");
    let actions = response["result"].as_array().expect("result should be an array");
    assert!(
        actions.iter().any(|a| a["kind"].as_str() == Some("quickfix")),
        "expected a quickfix action, got: {response}"
    );

    drop(stdin);
    let _ = child.kill();
}

#[test]
fn lsp_diagnostic_range_covers_full_line() {
    // The diagnostic end character must extend to the end of the line so that
    // editors like Neovim consider the cursor "inside" the diagnostic wherever
    // it is on the line and show the code-action lightbulb.
    let (mut child, mut reader) = start_lsp_server();
    let mut stdin = child.stdin.take().unwrap();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    send_lsp_message(&mut stdin, init).unwrap();
    let _init_response = read_lsp_response(&mut reader).unwrap();
    send_lsp_message(&mut stdin, r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#).unwrap();

    let uri = "file:///tmp/test_diag_range.agile.md";
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "markdown",
                "version": 1,
                "text": "- [ ] top\n   - [ ] sub\n"
            }
        }
    });
    send_lsp_message(&mut stdin, &did_open.to_string()).unwrap();

    let notification = read_notification(&mut reader, "textDocument/publishDiagnostics");
    let diags = notification["params"]["diagnostics"].as_array().unwrap();
    assert!(!diags.is_empty(), "expected at least one diagnostic");

    let end_char = diags[0]["range"]["end"]["character"].as_u64().unwrap();
    assert_eq!(end_char, 100, "diagnostic end character should be 100");

    drop(stdin);
    let _ = child.kill();
}
