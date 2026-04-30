//! LSP Protocol message types.
//!
//! Implements JSON-RPC 2.0 wrapper and LSP message types for communication
//! between the IDE client and our language server.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ─────────────────────────────────────────────────────────────────────────────
// JSON-RPC 2.0 Envelope
// ─────────────────────────────────────────────────────────────────────────────

/// JSON-RPC 2.0 request or notification (incoming message from client).
#[derive(Debug, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Number>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response (outgoing message to client).
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<serde_json::Number>, result: Value) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Number>, code: i32, message: String) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }

    pub fn notification(method: &str, params: Value) -> String {
        let notif = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        notif.to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LSP Initialize
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InitializeParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_uri: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InitializeResult {
    pub capabilities: ServerCapabilities,
}

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    /// The server can handle `textDocument/didOpen` notifications
    pub text_document_sync: i32,
}

// ─────────────────────────────────────────────────────────────────────────────
// LSP TextDocument Events
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DidOpenTextDocumentParams {
    pub text_document: TextDocumentItem,
}

#[derive(Debug, Deserialize)]
pub struct TextDocumentItem {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_id: Option<String>,
    pub version: i32,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct DidChangeTextDocumentParams {
    pub text_document: VersionedTextDocumentIdentifier,
    pub content_changes: Vec<TextDocumentContentChangeEvent>,
}

#[derive(Debug, Deserialize)]
pub struct VersionedTextDocumentIdentifier {
    pub uri: String,
    pub version: i32,
}

#[derive(Debug, Deserialize)]
pub struct TextDocumentContentChangeEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_length: Option<i32>,
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Position {
    pub line: i32,
    pub character: i32,
}

#[derive(Debug, Deserialize)]
pub struct DidCloseTextDocumentParams {
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// LSP Diagnostics
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PublishDiagnosticsParams {
    pub uri: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: i32,  // 1 = Error, 2 = Warning, 3 = Information, 4 = Hint
    pub code: String,
    pub source: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticRelatedInformation {
    pub location: Location,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_initialize_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootPath":"/tmp"}}"#;
        let msg: JsonRpcMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.method, "initialize");
        assert_eq!(msg.id, Some(serde_json::Number::from(1)));
    }

    #[test]
    fn serialize_initialize_response() {
        let response = JsonRpcResponse::success(
            Some(serde_json::Number::from(1)),
            json!({
                "capabilities": {
                    "textDocumentSync": 1
                }
            }),
        );
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains("\"result\""));
        assert!(serialized.contains("\"capabilities\""));
    }
}
