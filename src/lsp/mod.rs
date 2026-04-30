//! LSP (Language Server Protocol) server implementation.
//!
//! Provides real-time validation of `.agile.md` files through a JSON-RPC 2.0
//! interface over stdin/stdout. Integrates with the existing parser and checker
//! to provide diagnostics as users edit their task files.

pub mod protocol;
pub mod handler;

use std::io::{self, BufRead, Write};
use serde_json::Value;

use protocol::{JsonRpcMessage, JsonRpcResponse};
use handler::Handler;

/// Run the LSP server on stdin/stdout.
///
/// Reads JSON-RPC 2.0 messages from stdin, processes them, and writes
/// responses to stdout. Continues until shutdown request is received.
pub fn run() -> io::Result<()> {
    let mut handler = Handler::new();
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Parse JSON-RPC message
        let msg: JsonRpcMessage = match serde_json::from_str(&line) {
            Ok(msg) => msg,
            Err(_e) => {
                // Invalid JSON - send error response
                let error = JsonRpcResponse::error(None, -32700, "Parse error".to_string());
                writeln!(stdout.lock(), "{}", serde_json::to_string(&error)?)?;
                continue;
            }
        };

        // Handle message
        match msg.method.as_str() {
            "initialize" => {
                let result = handler.initialize(&msg);
                let response = JsonRpcResponse::success(msg.id, result);
                writeln!(stdout.lock(), "{}", serde_json::to_string(&response)?)?;
            }
            "initialized" => {
                // Notification - no response expected
                handler.initialized();
            }
            "textDocument/didOpen" => {
                handler.did_open(&msg);
            }
            "textDocument/didChange" => {
                handler.did_change(&msg);
            }
            "textDocument/didClose" => {
                handler.did_close(&msg);
            }
            "shutdown" => {
                handler.shutdown();
                let response = JsonRpcResponse::success(msg.id, Value::Null);
                writeln!(stdout.lock(), "{}", serde_json::to_string(&response)?)?;
                break;
            }
            "exit" => {
                // Exit notification - no response, just exit
                break;
            }
            _ => {
                // Unknown method
                let error = JsonRpcResponse::error(
                    msg.id,
                    -32601,
                    format!("Unknown method: {}", msg.method),
                );
                writeln!(stdout.lock(), "{}", serde_json::to_string(&error)?)?;
            }
        }
    }

    Ok(())
}
