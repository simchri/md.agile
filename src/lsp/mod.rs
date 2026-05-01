//! Minimal LSP server built on `tower-lsp`.
//!
//! Advertises basic capabilities, syncs document text in FULL mode, runs
//! the existing checker rules on every open/change to publish diagnostics,
//! and offers `quickfix` code actions for fixable diagnostics (E002).

pub mod logger;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::info;

use crate::{checker, parser, rules::{Issue, IssueData}};

struct Backend {
    client: Client,
    /// Latest text of every open document, keyed by URI.
    docs: Arc<RwLock<HashMap<Url, String>>>,
    /// Last diagnostics published for each URI, keyed by URI. Stored
    /// server-side so code_action can look them up without relying on the
    /// client echoing the `data` field back (Neovim strips it).
    diagnostics: Arc<RwLock<HashMap<Url, Vec<Diagnostic>>>>,
}

impl Backend {
    async fn validate(&self, uri: Url, text: &str, version: Option<i32>) {
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));
        let items = parser::parse(text, path);
        let diagnostics: Vec<Diagnostic> = checker::run(&items)
            .into_iter()
            .map(issue_to_diagnostic)
            .collect();
        self.diagnostics.write().await.insert(uri.clone(), diagnostics.clone());
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
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
        end:   Position { line, character: dash_col.max(1) },
    };

    let sev = match issue.code.chars().next() {
        Some('E') => DiagnosticSeverity::ERROR,
        Some('W') => DiagnosticSeverity::WARNING,
        _         => DiagnosticSeverity::ERROR,
    };

    let data = issue.data.as_ref().and_then(|d| serde_json::to_value(d).ok());

    Diagnostic {
        range,
        severity: Some(sev),
        code: Some(NumberOrString::String(issue.code)),
        source: Some("agilels".to_string()),
        message: match issue.help {
            Some(h) => format!("{}\n{}", format_message(issue.message), format_help(h)),
            None    => format_message(issue.message),
        },
        data,
        ..Diagnostic::default()
    }
}

/// Builds a `quickfix` code action for a single E002 diagnostic, or `None`
/// if the diagnostic is not auto-fixable.
///
/// Pure helper so we can unit-test it without driving the full LSP loop.
/// `doc_text` is the current content of the document; the edit replaces the
/// leading whitespace of the offending line with exactly `expected_indent`
/// spaces (read from `diagnostic.data`).
fn build_quickfix(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    // Only E002 is auto-fixable; E001 needs the user to decide intent.
    match &diagnostic.code {
        Some(NumberOrString::String(s)) if s == "E002" => {}
        _ => return None,
    }

    // Pull expected_indent out of the diagnostic's data payload.
    let data = diagnostic.data.as_ref()?;
    let issue_data: IssueData = serde_json::from_value(data.clone()).ok()?;
    let IssueData::WrongIndent { expected_indent } = issue_data;

    let line_idx = diagnostic.range.start.line as usize;
    let line_text = doc_text.lines().nth(line_idx)?;
    let current_indent = line_text.chars().take_while(|c| *c == ' ').count();

    let edit = TextEdit {
        range: Range {
            start: Position { line: diagnostic.range.start.line, character: 0 },
            end:   Position {
                line: diagnostic.range.start.line,
                character: current_indent as u32,
            },
        },
        new_text: " ".repeat(expected_indent),
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);

    Some(CodeAction {
        title: format!("Fix indentation: use {} spaces", expected_indent),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        command: None,
        disabled: None,
        data: None,
    })
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        info!("initialize");
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
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
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        info!("did_open {}", doc.uri);
        self.docs.write().await.insert(doc.uri.clone(), doc.text.clone());
        self.validate(doc.uri, &doc.text, Some(doc.version)).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        // FULL sync: a single change containing the entire new text.
        let Some(change) = params.content_changes.pop() else { return };
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

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> Result<Option<CodeActionResponse>> {
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
            .filter_map(|d| build_quickfix(d, &doc_text, &params.text_document.uri))
            .map(CodeActionOrCommand::CodeAction)
            .collect();

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    async fn shutdown(&self) -> Result<()> {
        info!("shutdown");
        Ok(())
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
            docs: Arc::new(RwLock::new(HashMap::new())),
            diagnostics: Arc::new(RwLock::new(HashMap::new())),
        });
        Server::new(stdin, stdout, socket).serve(service).await;
    });

    info!("LSP server stopped");
    Ok(())
}

fn ranges_overlap(a: &Range, b: &Range) -> bool {
    a.start <= b.end && b.start <= a.end
}

fn format_message(msg: String) -> String {
    format!("[{}]", msg.trim().to_string())
}

fn format_help(help: String) -> String {
    format!("HINT: {}", help.trim())
}

#[cfg(test)]
mod tests;

