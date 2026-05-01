//! LSP request and notification handlers.

use std::collections::HashMap;
use std::path::PathBuf;
use std::io::Write;
use serde_json::json;
use tracing::{info, debug};

use crate::lsp::protocol::{JsonRpcMessage, PublishDiagnosticsParams, Diagnostic, Range, Position, JsonRpcResponse};
use crate::parser::parse;
use crate::checker;

/// State manager for the LSP server.
///
/// Tracks open documents and server state across requests/notifications.
pub struct Handler {
    /// Map of document URI → document content
    documents: HashMap<String, String>,
    /// Whether server has been initialized
    initialized: bool,
    /// Whether shutdown has been requested
    shutdown: bool,
}

impl Handler {
    pub fn new() -> Self {
        Handler {
            documents: HashMap::new(),
            initialized: false,
            shutdown: false,
        }
    }

    /// Handle `initialize` request.
    ///
    /// Returns server capabilities describing what the server supports.
    pub fn initialize(&mut self, _msg: &JsonRpcMessage) -> serde_json::Value {
        debug!("Initializing handler");
        self.initialized = true;
        info!("Handler initialized");
        json!({
            "capabilities": {
                "textDocumentSync": {
                    "openClose": true,
                    "change": 1,  // 1 = Full document sync
                    "willSave": false,
                    "willSaveWaitUntil": false,
                    "didSave": false
                }
            },
            "serverInfo": {
                "name": "agilels",
                "version": env!("CARGO_PKG_VERSION")
            }
        })
    }

    /// Handle `initialized` notification.
    ///
    /// Called after client has received initialize response.
    pub fn initialized(&mut self) {
        // Could do startup work here (e.g., scan workspace)
    }

    /// Handle `textDocument/didOpen` notification.
    ///
    /// Stores the document and validates it.
    pub fn did_open(&mut self, msg: &JsonRpcMessage) {
        if let Some(params) = &msg.params {
            if let Ok(doc_uri) = params.get("textDocument")
                .and_then(|doc| doc.get("uri"))
                .and_then(|uri| uri.as_str())
                .ok_or(())
            {
                if let Ok(text) = params.get("textDocument")
                    .and_then(|doc| doc.get("text"))
                    .and_then(|t| t.as_str())
                    .ok_or(())
                {
                    info!("Document opened: {}", doc_uri);
                    debug!("Document content length: {} bytes", text.len());
                    self.documents.insert(doc_uri.to_string(), text.to_string());
                    self.validate_and_publish(doc_uri, text);
                }
            }
        }
    }

    /// Handle `textDocument/didChange` notification.
    ///
    /// Updates document and triggers re-validation.
    pub fn did_change(&mut self, msg: &JsonRpcMessage) {
        if let Some(params) = &msg.params {
            if let Ok(doc_uri) = params.get("textDocument")
                .and_then(|doc| doc.get("uri"))
                .and_then(|uri| uri.as_str())
                .ok_or(())
            {
                // For full document sync (textDocumentSync: 1), the text is the new full content
                if let Some(changes) = params.get("contentChanges").and_then(|c| c.as_array()) {
                    if let Some(first_change) = changes.first() {
                        if let Ok(text) = first_change
                            .get("text")
                            .and_then(|t| t.as_str())
                            .ok_or(())
                        {
                            info!("Document changed: {}", doc_uri);
                            debug!("New content length: {} bytes", text.len());
                            self.documents.insert(doc_uri.to_string(), text.to_string());
                            self.validate_and_publish(doc_uri, text);
                        }
                    }
                }
            }
        }
    }

    /// Handle `textDocument/didClose` notification.
    ///
    /// Forgets about the document.
    pub fn did_close(&mut self, msg: &JsonRpcMessage) {
        if let Some(params) = &msg.params {
            if let Ok(doc_uri) = params.get("textDocument")
                .and_then(|doc| doc.get("uri"))
                .and_then(|uri| uri.as_str())
                .ok_or(())
            {
                info!("Document closed: {}", doc_uri);
                self.documents.remove(doc_uri);
            }
        }
    }

    /// Handle `shutdown` request.
    ///
    /// Prepares the server for exit.
    pub fn shutdown(&mut self) {
        self.shutdown = true;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    /// Validate document and publish diagnostics to the client.
    ///
    /// Parses the document, runs the checker, and sends `textDocument/publishDiagnostics`
    /// notification with any issues found.
    fn validate_and_publish(&self, doc_uri: &str, text: &str) {
        // Extract filename from URI for parser (expects a path)
        let filename = doc_uri.split('/').last().unwrap_or("unknown.agile.md");
        let path = PathBuf::from(filename);
        
        debug!("Validating document: {}", filename);
        
        // Parse and check
        let items = parse(text, path);
        debug!("Parsed {} items", items.len());
        
        let issues = checker::run(&items);
        info!("Validation complete: found {} issues", issues.len());
        
        // Convert issues to LSP diagnostics
        let diagnostics: Vec<Diagnostic> = issues.into_iter().map(|issue| {
            debug!("Issue {}: {} at line {}", issue.code, issue.message, issue.location.line);
            // Convert 1-based line/column to 0-based
            let line = (issue.location.line as i32 - 1).max(0);
            let character = (issue.column as i32 - 1).max(0);
            
            Diagnostic {
                range: Range {
                    start: Position { line, character },
                    end: Position { line, character: character + 1 },
                },
                severity: 1, // Error
                code: issue.code,
                source: "agile".to_string(),
                message: issue.message,
                related_information: issue.help.map(|help| vec![
                    crate::lsp::protocol::DiagnosticRelatedInformation {
                        location: crate::lsp::protocol::Location {
                            uri: doc_uri.to_string(),
                            range: Range {
                                start: Position { line, character },
                                end: Position { line, character: character + 1 },
                            },
                        },
                        message: help,
                    },
                ]),
            }
        }).collect();
        
        // Publish diagnostics
        let params = PublishDiagnosticsParams {
            uri: doc_uri.to_string(),
            diagnostics,
        };
        let notification = JsonRpcResponse::notification(
            "textDocument/publishDiagnostics",
            serde_json::to_value(&params).unwrap_or(json!({})),
        );
        let _ = writeln!(std::io::stdout().lock(), "{}", notification);
        debug!("Diagnostics published for: {}", doc_uri);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_initialize_sets_flag() {
        let mut handler = Handler::new();
        assert!(!handler.is_initialized());

        let msg = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Number::from(1)),
            method: "initialize".to_string(),
            params: None,
        };

        handler.initialize(&msg);
        assert!(handler.is_initialized());
    }

    #[test]
    fn handler_shutdown_sets_flag() {
        let mut handler = Handler::new();
        assert!(!handler.is_shutdown());

        handler.shutdown();
        assert!(handler.is_shutdown());
    }
}
