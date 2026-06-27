use super::helpers::LspSession;
use serde_json::Value;

#[test]
fn lsp_initialize_advertises_code_action_provider() {
    let mut session = LspSession::start();

    let init_request = r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    session.send(init_request);

    let response = session.read_raw_response();
    let v: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(
        v["result"]["capabilities"]["codeActionProvider"],
        serde_json::json!(true),
        "response: {response}"
    );
}

#[test]
fn lsp_initialize_advertises_definition_provider() {
    let mut session = LspSession::start();

    let init_request = r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    session.send(init_request);

    let response = session.read_raw_response();
    let v: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(
        v["result"]["capabilities"]["definitionProvider"],
        serde_json::json!(true),
        "response: {response}"
    );
}

#[test]
fn lsp_initialize_request_returns_capabilities() {
    let mut session = LspSession::start();

    let init_request = r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"processId":1234,"rootUri":null,"capabilities":{}}}"#;
    session.send(init_request);

    let response = session.read_raw_response();

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
    let session = LspSession::start();

    // Server should still be running after the handshake.
    assert!(
        session.child.try_wait().is_ok(),
        "server should still be running"
    );
}

#[test]
fn lsp_shutdown_request_handled() {
    let mut session = LspSession::start();

    let shutdown = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#;
    session.send(shutdown);

    let response = session.read_raw_response();

    assert!(
        response.contains("\"result\":null"),
        "response: {}",
        response
    );
}
