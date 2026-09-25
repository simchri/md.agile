//! Minimal LSP server built on `tower-lsp`.
//!
//! Advertises basic capabilities, syncs document text in FULL mode, runs
//! the existing checker rules on every open/change to publish diagnostics,
//! and offers `quickfix` code actions for fixable diagnostics (E002/E003/E005).

pub mod goto_definition;
pub mod jump;
pub mod logger;
pub mod quickfix;
pub mod semantic_tokens;

use goto_definition::{
    assignment_name_at_position, find_assignment_line_in_config, find_property_line_in_config,
    property_name_at_position,
};
use jump::{
    find_highest_priority_open_task_in_workspace, find_my_highest_priority_open_task_in_workspace,
    highest_priority_open_task_line, my_highest_priority_open_task_line, next_my_open_task_after,
    next_open_task_after, previous_my_open_task_before, previous_open_task_before,
};
use quickfix::build_quickfixes;
use semantic_tokens::{TOKEN_TYPES, build_tokens};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use log::info;
use serde_json::Value;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::request::{GotoImplementationParams, GotoImplementationResponse};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::{
    checker,
    config::{Config, find_config_file_in, find_config_file_upwards},
    parser,
    rules::{Issue, ResolvedIdentity},
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
    /// Loads the `mdagile.toml`/`.mdagile.toml` config governing `path`
    /// (reporting/clearing the config-error notification as a side effect —
    /// see [`Self::report_config_error`]), falling back to an empty
    /// [`Config::default`] if no config file is found or it fails to parse.
    /// Returns `(config, config_load_failed)`; `config_load_failed` is `true`
    /// only when a config file exists but fails to load, so callers can tell
    /// "no config" apart from "broken config" (the placeholder default isn't
    /// a trustworthy "no properties/users declared" config in the latter
    /// case).
    async fn load_config(&self, config_path: Option<&Path>, path: &Path) -> (Config, bool) {
        match config_path {
            Some(config_path) => {
                let load_result = Config::load(config_path.parent().unwrap_or(config_path));
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
        }
    }

    /// The shared "jump to task" shape behind `goto_declaration` and
    /// `goto_implementation`: try `workspace_finder` across all task files
    /// under the project root first (if a root is known), then fall back to
    /// `doc_finder` on the currently open document. Returns the target
    /// `(uri, 0_based_line)`, or `None` if neither search finds a task.
    async fn jump_target(
        &self,
        uri: &Url,
        workspace_finder: impl Fn(&Path) -> Option<(PathBuf, u32)>,
        doc_finder: impl Fn(&str) -> Option<u32>,
    ) -> Option<(Url, u32)> {
        let target = if let Some(root) = self.root.read().await.as_ref() {
            workspace_finder(root)
                .and_then(|(path, line)| Url::from_file_path(path).ok().map(|u| (u, line)))
        } else {
            None
        };
        if let Some(t) = target {
            return Some(t);
        }
        let doc_text = self.docs.read().await.get(uri)?.clone();
        doc_finder(&doc_text).map(|line| (uri.clone(), line))
    }

    /// Resolves the config governing `uri` and the live git identity for
    /// "my task" features, reusing [`Self::load_config`] and
    /// [`checker::resolve_editor_identity`]. Returns `None` if no project
    /// root is known, or if an identity can't be determined (see
    /// [`checker::resolve_editor_identity`]) — both silent skip cases, since
    /// there's no terminal to warn on in the editor-integration path.
    async fn resolve_my_identity(
        &self,
        uri: &Url,
        path: &Path,
    ) -> Option<(Config, ResolvedIdentity)> {
        let root = self.root.read().await.clone()?;
        let config_path = self.resolve_config_path(uri).await;
        let (config, _) = self.load_config(config_path.as_deref(), path).await;
        let identity = checker::resolve_editor_identity(&root, &config)?;
        Some((config, identity))
    }

    /// The shared logic behind `goto_declaration`/`goto_implementation` and
    /// the `mdagile.jump.highestPriorityOpen`/`highestPriorityMy` commands:
    /// finds the highest-priority open task, restricted to tasks eligible
    /// for the caller's identity when `mine` is `true`. Returns the target
    /// `(uri, 0_based_line)`, or `None` if no matching task is found (or, for
    /// `mine`, if the caller's identity can't be resolved).
    async fn highest_priority_target(&self, uri: &Url, mine: bool) -> Option<(Url, u32)> {
        if !mine {
            return self
                .jump_target(
                    uri,
                    find_highest_priority_open_task_in_workspace,
                    highest_priority_open_task_line,
                )
                .await;
        }
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));
        let (config, identity) = self.resolve_my_identity(uri, &path).await?;
        self.jump_target(
            uri,
            |root| find_my_highest_priority_open_task_in_workspace(root, &identity, &config),
            |doc_text| my_highest_priority_open_task_line(doc_text, &identity, &config),
        )
        .await
    }

    /// The shared logic behind the `mdagile.jump.nextOpen`/`previousOpen`/
    /// `nextMy`/`previousMy` commands: finds the next/previous open task
    /// relative to `current_line` in `current_path`, preferring the live
    /// in-editor buffer for `current_path` (respecting unsaved edits, same
    /// as [`Self::jump_target`]) over its on-disk content, restricted to
    /// tasks eligible for the caller's identity when `mine` is `true`.
    /// Returns the target `(uri, 0_based_line)`, or `None` if no matching
    /// task is found (or, for `mine`, if the caller's identity can't be
    /// resolved, or no project root is known at all).
    async fn relative_target(
        &self,
        uri: &Url,
        current_path: &Path,
        current_line: u32,
        direction: jump::Direction,
        mine: bool,
    ) -> Option<(Url, u32)> {
        let root = self.root.read().await.clone()?;
        let current_doc_text = self.docs.read().await.get(uri).cloned();
        let (path, line) = if mine {
            let (config, identity) = self.resolve_my_identity(uri, current_path).await?;
            match direction {
                jump::Direction::Next => next_my_open_task_after(
                    &root,
                    current_path,
                    current_doc_text.as_deref(),
                    current_line,
                    &identity,
                    &config,
                ),
                jump::Direction::Previous => previous_my_open_task_before(
                    &root,
                    current_path,
                    current_doc_text.as_deref(),
                    current_line,
                    &identity,
                    &config,
                ),
            }
        } else {
            match direction {
                jump::Direction::Next => next_open_task_after(
                    &root,
                    current_path,
                    current_doc_text.as_deref(),
                    current_line,
                ),
                jump::Direction::Previous => previous_open_task_before(
                    &root,
                    current_path,
                    current_doc_text.as_deref(),
                    current_line,
                ),
            }
        }?;
        Url::from_file_path(path).ok().map(|u| (u, line))
    }

    /// Shows `target` (if any) via `window/showDocument`, taking editor
    /// focus, and returns whether a target was found — the shared response
    /// shape for every `mdagile.jump.*` command. Errors from the client
    /// (e.g. a client that doesn't support `window/showDocument`) are
    /// swallowed: the jump is best-effort, and the boolean return already
    /// tells the caller whether a target existed.
    async fn show_jump_target(&self, target: Option<(Url, u32)>) -> Result<Option<Value>> {
        let Some((uri, line)) = target else {
            return Ok(Some(Value::Bool(false)));
        };
        let selection = location_at_line(uri.clone(), line).range;
        let _ = self
            .client
            .show_document(ShowDocumentParams {
                uri,
                external: Some(false),
                take_focus: Some(true),
                selection: Some(selection),
            })
            .await;
        Ok(Some(Value::Bool(true)))
    }

    async fn validate(&self, uri: Url, text: &str, version: Option<i32>) {
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));
        let config_path = self.resolve_config_path(&uri).await;
        let (config, config_load_failed) = self.load_config(config_path.as_deref(), &path).await;
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

/// A zero-width [`Location`] at the start of `line` in `uri` — the target of
/// every "jump to task" LSP action, since tasks aren't given a meaningful
/// end position.
fn location_at_line(uri: Url, line: u32) -> Location {
    Location {
        uri,
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 0 },
        },
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

/// The `mdagile.jump.*` custom command names advertised via
/// `execute_command_provider` and dispatched by [`Backend::execute_command`].
///
/// Each command takes `arguments: [currentUri]` (the `highestPriority*`
/// commands) or `arguments: [currentUri, currentLine]` (the `next*`/
/// `previous*` commands, `currentLine` being the 0-based cursor line) and
/// jumps by issuing a `window/showDocument` request to the client — there's
/// no other client-visible way for a custom command to move the cursor.
const JUMP_COMMANDS: &[&str] = &[
    "mdagile.jump.highestPriorityOpen",
    "mdagile.jump.highestPriorityMy",
    "mdagile.jump.nextOpen",
    "mdagile.jump.previousOpen",
    "mdagile.jump.nextMy",
    "mdagile.jump.previousMy",
];

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
                declaration_provider: Some(DeclarationCapability::Simple(true)),
                implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: JUMP_COMMANDS.iter().map(|s| s.to_string()).collect(),
                    work_done_progress_options: Default::default(),
                }),
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

    async fn goto_declaration(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;

        let Some((target_uri, line)) = self.highest_priority_target(uri, false).await else {
            return Ok(None);
        };

        Ok(Some(GotoDefinitionResponse::Scalar(location_at_line(
            target_uri, line,
        ))))
    }

    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> Result<Option<GotoImplementationResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;

        let Some((target_uri, line)) = self.highest_priority_target(uri, true).await else {
            return Ok(None);
        };

        Ok(Some(GotoImplementationResponse::Scalar(location_at_line(
            target_uri, line,
        ))))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        // arguments[0] is always the requesting document's URI (as a
        // string); the next*/previous* commands additionally take
        // arguments[1], the 0-based cursor line.
        let Some(uri) = params
            .arguments
            .first()
            .and_then(|v| v.as_str())
            .and_then(|s| Url::parse(s).ok())
        else {
            return Ok(Some(Value::Bool(false)));
        };
        let current_path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));
        let current_line = params.arguments.get(1).and_then(|v| v.as_u64());

        let target = match params.command.as_str() {
            "mdagile.jump.highestPriorityOpen" => self.highest_priority_target(&uri, false).await,
            "mdagile.jump.highestPriorityMy" => self.highest_priority_target(&uri, true).await,
            "mdagile.jump.nextOpen"
            | "mdagile.jump.nextMy"
            | "mdagile.jump.previousOpen"
            | "mdagile.jump.previousMy" => {
                let Some(current_line) = current_line else {
                    return Ok(Some(Value::Bool(false)));
                };
                let mine = params.command.ends_with("My");
                let direction = if params.command.starts_with("mdagile.jump.next") {
                    jump::Direction::Next
                } else {
                    jump::Direction::Previous
                };
                self.relative_target(&uri, &current_path, current_line as u32, direction, mine)
                    .await
            }
            _ => return Err(tower_lsp::jsonrpc::Error::method_not_found()),
        };

        self.show_jump_target(target).await
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
