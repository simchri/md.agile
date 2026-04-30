//! LSP request and notification handlers.

use std::collections::HashMap;
use serde_json::json;

use crate::lsp::protocol::JsonRpcMessage;

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
        self.initialized = true;
        json!({
            "capabilities": {
                "textDocumentSync": 1  // 1 = Full document sync
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
    /// Stores the document and could trigger initial validation.
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
                    self.documents.insert(doc_uri.to_string(), text.to_string());
                    // Could validate here and publish diagnostics
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
                            self.documents.insert(doc_uri.to_string(), text.to_string());
                            // Could validate here and publish diagnostics
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
