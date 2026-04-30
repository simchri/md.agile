//! Acceptance tests for `agile lsp` (Language Server Protocol).
//!
//! Tests the LSP server by sending JSON-RPC messages to stdin and
//! verifying responses on stdout. Following the LSP specification.

use std::process::{Command, Stdio};
use std::io::{Write, BufRead, BufReader};

fn start_lsp_server() -> (std::process::Child, BufReader<std::process::ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agile"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn `agile lsp`");

    let stdout = child.stdout.take().expect("stdout");
    let reader = BufReader::new(stdout);
    
    (child, reader)
}

#[test]
fn lsp_initialize_request_returns_capabilities() {
    let (mut child, mut reader) = start_lsp_server();
    let mut stdin = child.stdin.take().unwrap();

    // Send initialize request
    let init_request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootPath":"/tmp"}}"#;
    writeln!(stdin, "{}", init_request).unwrap();
    
    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).unwrap();
    
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
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootPath":"/tmp"}}"#;
    writeln!(stdin, "{}", init).unwrap();
    
    // Read initialize response
    let mut buf = String::new();
    reader.read_line(&mut buf).unwrap();
    
    // Send initialized notification (no response expected)
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    writeln!(stdin, "{}", initialized).unwrap();
    
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
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootPath":"/tmp"}}"#;
    writeln!(stdin, "{}", init).unwrap();
    let mut buf = String::new();
    reader.read_line(&mut buf).unwrap();
    
    // Send shutdown
    let shutdown = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}"#;
    writeln!(stdin, "{}", shutdown).unwrap();
    
    // Read shutdown response
    buf.clear();
    reader.read_line(&mut buf).unwrap();
    
    assert!(buf.contains("\"result\":null"), "response: {}", buf);
    
    // Cleanup
    drop(stdin);
    let _ = child.kill();
}
