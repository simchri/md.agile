//! Minimal LSP server built on `tower-lsp`.
//!
//! Advertises basic capabilities, syncs document text in FULL mode, and runs
//! the existing checker rules on every open/change to publish diagnostics.

pub mod logger;

use std::path::PathBuf;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::info;

use crate::{checker, parser, rules::Issue};

struct Backend {
    client: Client,
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
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(issue.code)),
        source: Some("agilels".to_string()),
        message: match issue.help {
            Some(h) => format!("{}\n{}", issue.message, h),
            None    => issue.message,
        },
        ..Diagnostic::default()
    }
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
        self.validate(doc.uri, &doc.text, Some(doc.version)).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        // FULL sync: a single change containing the entire new text.
        let Some(change) = params.content_changes.pop() else { return };
        info!("did_change {}", params.text_document.uri);
        self.validate(
            params.text_document.uri,
            &change.text,
            Some(params.text_document.version),
        )
        .await;
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
        let (service, socket) = LspService::new(|client| Backend { client });
        Server::new(stdin, stdout, socket).serve(service).await;
    });

    info!("LSP server stopped");
    Ok(())
}
