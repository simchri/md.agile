//! Minimal LSP server built on `tower-lsp`.
//!
//! Advertises basic capabilities, syncs document text in FULL mode, runs
//! the existing checker rules on every open/change to publish diagnostics,
//! and offers `quickfix` code actions for fixable diagnostics (E002/E003/E005).

pub mod goto_definition;
pub mod logger;
pub mod quickfix;
pub mod semantic_tokens;

use goto_definition::{
    assignment_name_at_position, find_assignment_line_in_config, find_property_line_in_config,
    property_name_at_position,
};
use quickfix::build_quickfixes;
use semantic_tokens::{TOKEN_TYPES, build_tokens};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use log::info;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::{checker, config::Config, parser, rules::Issue};

struct Backend {
    client: Client,
    /// Project root received from the editor's initialize request.
    root: Arc<RwLock<Option<PathBuf>>>,
    /// Latest text of every open document, keyed by URI.
    docs: Arc<RwLock<HashMap<Url, String>>>,
    /// Last diagnostics published for each URI, keyed by URI. Stored
    /// server-side so code_action can look them up without relying on the
    /// client echoing the `data` field back (Neovim strips it).
    diagnostics: Arc<RwLock<HashMap<Url, Vec<Diagnostic>>>>,
    /// Last known modification time of the config file (for polling).
    config_mtime: Arc<RwLock<Option<SystemTime>>>,
}

impl Backend {
    async fn validate(&self, uri: Url, text: &str, version: Option<i32>) {
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));
        let config = match self.resolve_config_path(&uri).await {
            Some(config_path) => {
                Config::load(config_path.parent().unwrap_or(config_path.as_path()))
                    .unwrap_or_default()
            }
            None => {
                log::warn!(
                    "No config file found for {}. Falling back to empty config.",
                    path.display()
                );
                Config::default()
            }
        };
        let items = parser::parse(text, path);
        let diagnostics: Vec<Diagnostic> = checker::run(&items, &config)
            .into_iter()
            .map(issue_to_diagnostic)
            .collect();
        self.diagnostics
            .write()
            .await
            .insert(uri.clone(), diagnostics.clone());
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }

    /// Resolve the config file path for the given document URI.
    ///
    /// This is the single source of truth for config discovery in the LSP server.
    /// Uses the editor-supplied project root when set; otherwise walks up from
    /// the document's directory.
    async fn resolve_config_path(&self, uri: &Url) -> Option<PathBuf> {
        let root = self.root.read().await;
        if let Some(root) = root.as_ref() {
            Self::find_config_file(root)
        } else {
            let file_path = uri
                .to_file_path()
                .unwrap_or_else(|_| PathBuf::from(uri.path()));
            config_file_for_path(&file_path)
        }
    }

    /// Find the config file path for the given root directory.
    fn find_config_file(root: &std::path::Path) -> Option<PathBuf> {
        for name in &["mdagile.toml", ".mdagile.toml"] {
            let path = root.join(name);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    /// Check if config file has been modified since last check, and re-validate all docs if so.
    async fn check_config_changed(&self) {
        let root = match self.root.read().await.as_ref() {
            Some(r) => r.clone(),
            None => return,
        };

        let config_path = match Self::find_config_file(&root) {
            Some(p) => p,
            None => return,
        };

        let current_mtime = std::fs::metadata(&config_path)
            .and_then(|m| m.modified())
            .ok();

        let mut last_mtime = self.config_mtime.write().await;
        if current_mtime.is_some() && *last_mtime != current_mtime {
            *last_mtime = current_mtime;

            // Config changed, re-validate all open documents.
            let docs = self.docs.read().await.clone();
            for (uri, text) in docs {
                self.validate(uri, &text, None).await;
            }
        }
    }
}

fn issue_to_diagnostic(issue: Issue) -> Diagnostic {
    // Parser uses 1-based lines/columns; LSP uses 0-based.
    // For E001 (orphaned indented task), `column` is the 1-based column of the
    // dash, so the leading whitespace runs from column 0 to column-1.
    let line = issue.location.line.saturating_sub(1) as u32;
    let dash_col = issue.column.saturating_sub(1) as u32;
    let range = Range {
        start: Position { line, character: 0 },
        end: Position {
            line,
            character: dash_col.max(1),
        },
    };

    let sev = match issue.code.as_str().chars().next() {
        Some('E') => DiagnosticSeverity::ERROR,
        Some('W') => DiagnosticSeverity::WARNING,
        _ => DiagnosticSeverity::ERROR,
    };

    let data = issue
        .data
        .as_ref()
        .and_then(|d| serde_json::to_value(d).ok());

    let head = format_message(issue.message);
    let head = if quickfix::has_quickfix(issue.code) {
        format!("{head} (fix avail.)")
    } else {
        head
    };
    let message = match issue.help {
        Some(h) => format!("{}\n{}", head, format_help(h)),
        None => head,
    };

    Diagnostic {
        range,
        severity: Some(sev),
        code: Some(NumberOrString::String(issue.code.as_str().to_string())),
        source: Some("agilels".to_string()),
        message,
        data,
        ..Diagnostic::default()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let root = params.root_uri.and_then(|u| u.to_file_path().ok());
        info!("initialize, root: {:?}", root);
        *self.root.write().await = root;
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: TOKEN_TYPES.to_vec(),
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "agilels".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("initialized");
        self.client
            .log_message(MessageType::INFO, "agilels ready")
            .await;

        // Spawn a background task to poll the config file once per second.
        let client = self.client.clone();
        let root = self.root.clone();
        let docs = self.docs.clone();
        let diagnostics = self.diagnostics.clone();
        let config_mtime = self.config_mtime.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                let backend = Backend {
                    client: client.clone(),
                    root: root.clone(),
                    docs: docs.clone(),
                    diagnostics: diagnostics.clone(),
                    config_mtime: config_mtime.clone(),
                };
                backend.check_config_changed().await;
            }
        });
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        info!("did_open {}", doc.uri);
        self.docs
            .write()
            .await
            .insert(doc.uri.clone(), doc.text.clone());
        self.validate(doc.uri, &doc.text, Some(doc.version)).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        // FULL sync: a single change containing the entire new text.
        let Some(change) = params.content_changes.pop() else {
            return;
        };
        info!("did_change {}", params.text_document.uri);
        self.docs
            .write()
            .await
            .insert(params.text_document.uri.clone(), change.text.clone());
        self.validate(
            params.text_document.uri,
            &change.text,
            Some(params.text_document.version),
        )
        .await;
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let doc_text = match self.docs.read().await.get(&params.text_document.uri) {
            Some(t) => t.clone(),
            None => return Ok(None),
        };

        // Use the server's own stored diagnostics rather than what the client
        // echoes back in context.diagnostics — clients like Neovim strip the
        // `data` field, which build_quickfix requires.
        let stored = self.diagnostics.read().await;
        let diags = stored
            .get(&params.text_document.uri)
            .cloned()
            .unwrap_or_default();
        drop(stored);

        let actions: Vec<CodeActionOrCommand> = diags
            .iter()
            .filter(|d| ranges_overlap(&d.range, &params.range))
            .flat_map(|d| build_quickfixes(d, &doc_text, &params.text_document.uri))
            .map(CodeActionOrCommand::CodeAction)
            .collect();

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;
        let doc_text = match self.docs.read().await.get(uri) {
            Some(t) => t.clone(),
            None => return Ok(None),
        };
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));
        let items = parser::parse(&doc_text, path);
        let data = build_tokens(&items);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }

    async fn shutdown(&self) -> Result<()> {
        info!("shutdown");
        Ok(())
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let doc_text = match self.docs.read().await.get(uri) {
            Some(t) => t.clone(),
            None => return Ok(None),
        };

        // Determine what name is under the cursor and which config finder to use.
        // Property (#name) is tried first; assignment (@name) second.
        type Finder = fn(&str, &str) -> Option<u32>;
        let (name, finder): (String, Finder) = if let Some(n) =
            property_name_at_position(&doc_text, pos.line, pos.character)
        {
            (n, find_property_line_in_config)
        } else if let Some(n) = assignment_name_at_position(&doc_text, pos.line, pos.character) {
            (n, find_assignment_line_in_config)
        } else {
            return Ok(None);
        };

        let config_path = match self.resolve_config_path(uri).await {
            Some(p) => p,
            None => return Ok(None),
        };

        let config_uri = match Url::from_file_path(&config_path) {
            Ok(u) => u,
            Err(_) => return Ok(None),
        };

        // Use the in-editor buffer if the config file is open (respects unsaved edits).
        let config_text = {
            let docs = self.docs.read().await;
            if let Some(t) = docs.get(&config_uri) {
                t.clone()
            } else {
                drop(docs);
                match std::fs::read_to_string(&config_path) {
                    Ok(t) => t,
                    Err(_) => return Ok(None),
                }
            }
        };

        let line = match finder(&config_text, &name) {
            Some(l) => l,
            None => return Ok(None),
        };

        let location = Location {
            uri: config_uri,
            range: Range {
                start: Position { line, character: 0 },
                end: Position { line, character: 0 },
            },
        };

        Ok(Some(GotoDefinitionResponse::Scalar(location)))
    }
}

/// Run the LSP server on stdin/stdout.
pub fn run() -> std::io::Result<()> {
    let log_path = logger::init_logging()?;
    info!("LSP server starting, logging to: {:?}", log_path);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(|client| Backend {
            client,
            root: Arc::new(RwLock::new(None)),
            docs: Arc::new(RwLock::new(HashMap::new())),
            diagnostics: Arc::new(RwLock::new(HashMap::new())),
            config_mtime: Arc::new(RwLock::new(None)),
        });
        Server::new(stdin, stdout, socket).serve(service).await;
    });

    info!("LSP server stopped");
    Ok(())
}

fn ranges_overlap(a: &Range, b: &Range) -> bool {
    a.start.line <= b.end.line && b.start.line <= a.end.line
}

/// Walk up from `file_path`'s directory to find the nearest mdagile config file.
fn config_file_for_path(file_path: &std::path::Path) -> Option<PathBuf> {
    let mut dir = file_path.parent()?;
    loop {
        for name in &["mdagile.toml", ".mdagile.toml"] {
            let p = dir.join(name);
            if p.exists() {
                return Some(p);
            }
        }
        dir = dir.parent()?;
    }
}

fn format_message(msg: String) -> String {
    format!("[{}]", msg.trim().to_string())
}

fn format_help(help: String) -> String {
    format!("HINT: {}", help.trim())
}

#[cfg(test)]
mod tests;
