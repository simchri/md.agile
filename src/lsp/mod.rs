//! LSP (Language Server Protocol) server implementation.
//!
//! Provides real-time validation of `.agile.md` files through a JSON-RPC 2.0
//! interface over stdin/stdout. Integrates with the existing parser and checker
//! to provide diagnostics as users edit their task files.

pub mod protocol;
pub mod handler;
pub mod logger;

use std::io::{self, BufRead, BufReader, Write};
use std::collections::HashMap;
use serde_json::Value;
use tracing::{info, debug, warn, error};

use protocol::{JsonRpcMessage, JsonRpcResponse};
use handler::Handler;

/// Read LSP message with Content-Length header.
fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut headers = HashMap::new();
    let mut header_line = String::new();

    // Read headers until blank line
    loop {
        header_line.clear();
        let n = reader.read_line(&mut header_line)?;
        if n == 0 {
            // EOF
            return Ok(None);
        }

        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            // Blank line marks end of headers
            break;
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }

    // Get Content-Length
    let content_length: usize = headers
        .get("content-length")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing Content-Length"))?;

    // Read message body
    let mut message = vec![0u8; content_length];
    reader.read_exact(&mut message)?;

    Ok(Some(String::from_utf8_lossy(&message).to_string()))
}

/// Write LSP response with Content-Length header.
fn write_response<W: Write>(writer: &mut W, response: &str) -> io::Result<()> {
    writeln!(writer, "Content-Length: {}", response.len())?;
    writeln!(writer, "Content-Type: application/vscode-jsonrpc; charset=utf-8")?;
    writeln!(writer)?;
    write!(writer, "{}", response)?;
    writer.flush()?;
    Ok(())
}

/// Run the LSP server on stdin/stdout.
///
/// Reads JSON-RPC 2.0 messages from stdin (with LSP Content-Length headers),
/// processes them, and writes responses to stdout. Continues until shutdown
/// request is received.
pub fn run() -> io::Result<()> {
    let log_path = logger::init_logging()?;
    info!("LSP server starting, logging to: {:?}", log_path);

    let mut handler = Handler::new();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = io::stdout();

    info!("Waiting for messages...");

    loop {
        match read_message(&mut reader) {
            Ok(Some(message)) => {
                debug!("Received message: {}", message);

                // Parse JSON-RPC message
                let msg: JsonRpcMessage = match serde_json::from_str::<JsonRpcMessage>(&message) {
                    Ok(msg) => {
                        debug!("Parsed JSON-RPC message: method={}, id={:?}", msg.method, msg.id);
                        msg
                    }
                    Err(e) => {
                        warn!("Failed to parse JSON-RPC message: {}", e);
                        let error = JsonRpcResponse::error(None, -32700, "Parse error".to_string());
                        let response_str = serde_json::to_string(&error)?;
                        write_response(&mut stdout, &response_str)?;
                        continue;
                    }
                };

                // Handle message
                match msg.method.as_str() {
                    "initialize" => {
                        info!("Handling initialize request (id: {:?})", msg.id);
                        let result = handler.initialize(&msg);
                        let response = JsonRpcResponse::success(msg.id, result);
                        let response_str = serde_json::to_string(&response)?;
                        write_response(&mut stdout, &response_str)?;
                        info!("Initialize response sent");
                    }
                    "initialized" => {
                        info!("Handling initialized notification");
                        handler.initialized();
                    }
                    "textDocument/didOpen" => {
                        info!("Handling textDocument/didOpen");
                        handler.did_open(&msg);
                    }
                    "textDocument/didChange" => {
                        info!("Handling textDocument/didChange");
                        handler.did_change(&msg);
                    }
                    "textDocument/didClose" => {
                        info!("Handling textDocument/didClose");
                        handler.did_close(&msg);
                    }
                    "shutdown" => {
                        info!("Handling shutdown request (id: {:?})", msg.id);
                        let response = JsonRpcResponse::success(msg.id, Value::Null);
                        let response_str = serde_json::to_string(&response)?;
                        write_response(&mut stdout, &response_str)?;
                        handler.shutdown();
                        info!("Shutdown response sent, exiting");
                        break;
                    }
                    "exit" => {
                        info!("Handling exit notification");
                        break;
                    }
                    _ => {
                        warn!("Unknown method: {}", msg.method);
                        let error = JsonRpcResponse::error(
                            msg.id,
                            -32601,
                            format!("Unknown method: {}", msg.method),
                        );
                        let response_str = serde_json::to_string(&error)?;
                        write_response(&mut stdout, &response_str)?;
                    }
                }
            }
            Ok(None) => {
                info!("End of input, shutting down");
                break;
            }
            Err(e) => {
                error!("Error reading message: {}", e);
                break;
            }
        }
    }

    info!("LSP server stopped");
    Ok(())
}
