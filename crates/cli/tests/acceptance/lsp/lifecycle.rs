use super::helpers::{LspSession, read_lsp_response, send_lsp_message};
use serde_json::Value;

const INIT_REQUEST: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;

#[test]
fn lsp_initialize_advertises_code_action_provider() {
    let mut session = LspSession::start_raw();
    send_lsp_message(&mut session.stdin, INIT_REQUEST).unwrap();

    let response = read_lsp_response(&mut session.reader).unwrap();
    let v: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(
        v["result"]["capabilities"]["codeActionProvider"],
        serde_json::json!(true),
        "response: {response}"
    );
}

#[test]
fn lsp_initialize_advertises_definition_provider() {
    let mut session = LspSession::start_raw();
    send_lsp_message(&mut session.stdin, INIT_REQUEST).unwrap();

    let response = read_lsp_response(&mut session.reader).unwrap();
    let v: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(
        v["result"]["capabilities"]["definitionProvider"],
        serde_json::json!(true),
        "response: {response}"
    );
}

#[test]
fn lsp_initialize_request_returns_capabilities() {
    let mut session = LspSession::start_raw();
    send_lsp_message(&mut session.stdin, INIT_REQUEST).unwrap();

    let response = read_lsp_response(&mut session.reader).unwrap();

    assert!(
        response.contains("\"jsonrpc\":\"2.0\""),
        "response: {}",
        response
    );
    assert!(response.contains("\"result\""), "response: {}", response);
    assert!(
        response.contains("\"capabilities\""),
        "response: {}",
        response
    );
}

#[test]
fn lsp_initialized_notification_accepted() {
    let mut session = LspSession::start_raw();
    send_lsp_message(&mut session.stdin, INIT_REQUEST).unwrap();
    read_lsp_response(&mut session.reader).unwrap();
    send_lsp_message(&mut session.stdin, INITIALIZED).unwrap();

    // Server should still be running after the handshake.
    assert!(
        session.child.try_wait().is_ok(),
        "server should still be running"
    );
}

#[test]
fn lsp_shutdown_request_handled() {
    let mut session = LspSession::start_raw();
    send_lsp_message(&mut session.stdin, INIT_REQUEST).unwrap();
    read_lsp_response(&mut session.reader).unwrap();
    send_lsp_message(&mut session.stdin, INITIALIZED).unwrap();

    let shutdown = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#;
    send_lsp_message(&mut session.stdin, shutdown).unwrap();

    let response = session.read_response(2);

    assert!(response["result"].is_null(), "response: {}", response);
}
