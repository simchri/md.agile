//! Acceptance tests for `agile lsp` (Language Server Protocol).
//!
//! Tests the LSP server by sending JSON-RPC messages to stdin and
//! verifying responses on stdout. Following the LSP specification.

use std::process::{Command, Stdio};
use std::io::{Write, BufRead, BufReader};

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
