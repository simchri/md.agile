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

use crate::{
    checker,
    config::{Config, find_config_file_in, find_config_file_upwards},
    parser,
    rules::Issue,
};

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
    /// The current config-load error message, if the last attempt to load
    /// `mdagile.toml`/`.mdagile.toml` failed (invalid TOML, conflicting
    /// config files, property/group/identity validation). `None` when the
    /// config loaded successfully or no config file exists. Tracked so
    /// `validate()` only pops a `show_message` notification on the
    /// broken → fixed → broken *transition*, not on every keystroke while
    /// the config stays broken.
    config_error: Arc<RwLock<Option<String>>>,
}

impl Backend {
    async fn validate(&self, uri: Url, text: &str, version: Option<i32>) {
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));
        let config_path = self.resolve_config_path(&uri).await;
        let (config, config_load_failed) = match &config_path {
            Some(config_path) => {
                let load_result =
                    Config::load(config_path.parent().unwrap_or(config_path.as_path()));
                match load_result {
                    Ok(c) => {
                        self.clear_config_error().await;
                        (c, false)
                    }
                    Err(e) => {
                        self.report_config_error(e.to_string()).await;
                        (Config::default(), true)
                    }
                }
            }
            None => {
                log::warn!(
                    "No config file found for {}. Falling back to empty config.",
                    path.display()
                );
                self.clear_config_error().await;
                (Config::default(), false)
            }
        };
        let items = parser::parse(text, path.clone());
        // If mdagile.toml failed to load, `config` above is just an empty
        // placeholder, not a real "no properties/users declared" config —
        // running the config-dependent checks against it would report every
        // #marker/@marker as spuriously undefined. Only run the checks that
        // don't need a trustworthy config; the config_error_diagnostic
        // (added below) already explains why nothing else was checked.
        let mut issues = if config_load_failed {
            checker::run_config_independent(&items)
        } else {
            checker::run(&items, &config)
        };
        // The E013 assignment/completion check needs a project root to run git
        // commands from; reuse the config file's directory (same root the CLI
        // uses for `find_task_files`), falling back to the editor-supplied
        // workspace root if no config file was found. Also depends on
        // `[Users.X]`/`[Groups.X]` declarations, so it's skipped for the same
        // reason as the config-dependent rule checks above.
        let git_root = config_path
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .or(self.root.read().await.clone());
        if let (Some(root), false) = (git_root, config_load_failed) {
            issues.extend(checker::check_authorization_for_document(
                &root, &path, text, &config,
            ));
        }
        let mut diagnostics: Vec<Diagnostic> =
            issues.into_iter().map(issue_to_diagnostic).collect();
        if let Some(message) = self.config_error.read().await.as_ref() {
            diagnostics.push(config_error_diagnostic(message));
        }
        self.diagnostics
            .write()
            .await
            .insert(uri.clone(), diagnostics.clone());
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }

    /// Records that config loading failed with `message`, and — only on the
    /// transition into this error (or into a *different* error message) —
    /// pops a visible `window/showMessage` notification. Repeated
    /// `validate()` calls while the same error persists (e.g. one per
    /// keystroke) don't re-notify.
    async fn report_config_error(&self, message: String) {
        let mut current = self.config_error.write().await;
        if current.as_deref() != Some(message.as_str()) {
            self.client
                .show_message(
                    MessageType::ERROR,
                    format!("mdagile config error: {message}"),
                )
                .await;
        }
        *current = Some(message);
    }

    /// Clears any previously-recorded config-load error (config is now valid,
    /// or no config file exists at all).
    async fn clear_config_error(&self) {
        let mut current = self.config_error.write().await;
        *current = None;
    }

    /// Resolve the config file path for the given document URI.
    ///
    /// This is the single source of truth for config discovery in the LSP server.
    /// Uses the editor-supplied project root when set; otherwise walks up from
    /// the document's directory.
    async fn resolve_config_path(&self, uri: &Url) -> Option<PathBuf> {
        let root = self.root.read().await;
        if let Some(root) = root.as_ref() {
            find_config_file_in(root)
        } else {
            let file_path = uri
                .to_file_path()
                .unwrap_or_else(|_| PathBuf::from(uri.path()));
            let dir = file_path.parent()?;
            find_config_file_upwards(dir)
        }
    }

    /// Check if config file has been modified since last check, and re-validate all docs if so.
    async fn check_config_changed(&self) {
        let root = match self.root.read().await.as_ref() {
            Some(r) => r.clone(),
            None => return,
        };

        let config_path = match find_config_file_in(&root) {
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

/// A synthetic diagnostic (not tied to a specific line of the document) that
/// surfaces a broken `mdagile.toml`/`.mdagile.toml` load. Placed at the top
/// of whichever document is being validated, since there's no
/// document-independent way to report a workspace-level problem over LSP.
fn config_error_diagnostic(message: &str) -> Diagnostic {
    let range = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 1,
        },
    };
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        source: Some("agilels".to_string()),
        message: format!(
            "mdagile config error: {message}\nProperty/assignment/completion checks are disabled until mdagile.toml is fixed."
        ),
        data: None,
        ..Diagnostic::default()
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
        let config_error = self.config_error.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                let backend = Backend {
                    client: client.clone(),
                    root: root.clone(),
                    docs: docs.clone(),
                    diagnostics: diagnostics.clone(),
                    config_mtime: config_mtime.clone(),
                    config_error: config_error.clone(),
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
            config_error: Arc::new(RwLock::new(None)),
        });
        Server::new(stdin, stdout, socket).serve(service).await;
    });

    info!("LSP server stopped");
    Ok(())
}

fn ranges_overlap(a: &Range, b: &Range) -> bool {
    a.start.line <= b.end.line && b.start.line <= a.end.line
}

fn format_message(msg: String) -> String {
    format!("[{}]", msg.trim().to_string())
}

fn format_help(help: String) -> String {
    format!("HINT: {}", help.trim())
}

#[cfg(test)]
mod tests;
