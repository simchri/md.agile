- [x] post initial vision!
- [x] refine and clean up

# More Cli stuff
- [x] for any "list" command, add arguments "--next <count>" / "-n <count>"  show only the first <count> entries of whatever is listed.
- [x] idem "--last" ...

- [x] add subcommand (alias) "agile list tasks" should do the same as "agile list"

- [x] data structures suitable for parsing
- [x] implement a proper parser for tasks
- [x] integrate the parser into existing functions
- [x] next task uses parser, "location" added to each task struct

## small cli fixes
- [x] "tasks" is an alias for task subcommand

## First basic checks
- [x] agile check subcommand created
- [x] Check for wrongly indented task (task that is surrounded by newlines but indented like a subtaks)
any operation that parses tasks lists should immediately stop on encountering this error. The error should be printed, including file path and line number

- [x] iterate on the first error message
  - [x] proper coloring
  - [x] indenting
  - [x] other highlighting
  Bottom Line: It should be a really ergonomic, nicely readable error message, but which could also be parsed (machine readable)


## Fixes
- [x] clarify the command name - is it "agile" or "mdagile". The produced binary seems to be "mdagile". Check again what is says in the "vision" file, then fix accordingly
  Binary is now explicitly named "agile" via [[bin]] in Cargo.toml, matching vision.md

## First Minimal Language Server
- [x] hello world language server mode implemented into cli tool, command `agile `
- [x] wrongly indented task check is working in lsp
- [ ] testing for first lsp feature

- [ ] refactor and understand


## More basic checks (recommended next: E002 & E003)

- [ ] E002 + E003: Property & Assignment validation
  Detect undefined #property markers and @user/@group assignments
  - [ ] Read mdagile.toml config in checker; pass config to rules
  - [ ] E002: Detect undefined #property markers in tasks
    - [ ] Implement fuzzy matching to suggest close matches (typo detection)
    - [ ] Test with common typos: #Feature, #feat, etc.
  - [ ] E003: Detect undefined @user and @group assignments
    - [ ] Suggest close matches for misspelled names
    - [ ] Handle OR connections: @markus or @josh
  - [ ] Update error formatter for new error codes

- [ ] E004: Missing required subtasks
  Detect when a task has a property (e.g. #feature) but lacks required subtasks
  - [ ] Match quoted subtasks in tasks against property definitions from mdagile.toml
  - [ ] Handle multiple properties on same task
  - [ ] Handle nested properties (e.g., #feature that includes #review)
  - [ ] Provide helpful error with list of missing subtasks
  - [ ] Tests: single property, multiple properties, nested properties

- [ ] E005: Invalid order markers
  Detect duplicate order numbers, gaps, or malformed ordering syntax
  - [ ] Validate no duplicate ranks (e.g., two "2." markers)
  - [ ] Detect gaps in sequence (1, 3, skip 2)
  - [ ] Ensure ordering is only at same sibling level
  - [ ] Tests for various invalid orderings

- [ ] Data integrity: Incomplete parent tasks warning
  Warn when a parent marked done [x] still has [ ] children
  - [ ] Decide: warning (exit 0) vs error (exit 1)?
  - [ ] Consider: add --strict flag to promote warnings to errors


## More CLI features
- [ ] create a global overview of the CLI structure
  - [ ] list of subcommands and their functions
  - [ ] most important flags to each subcommand


## Language Server Protocol (LSP) Support

- [ ] LSP Phase 1: Core Foundation (Hello World)
  Entry point: `agile lsp` (stdin/stdout JSON-RPC)
  - [ ] Create src/lsp/protocol.rs — LSP message types (serde)
    - [ ] InitializeRequest/Response
    - [ ] DidOpenTextDocument / DidChangeTextDocument notifications
    - [ ] PublishDiagnosticsNotification
    - [ ] JsonRpc message wrapper
  - [ ] Create src/lsp/mod.rs — Main server loop
    - [ ] Read JSON-RPC from stdin
    - [ ] Dispatch to handlers
    - [ ] Write responses to stdout
  - [ ] Create src/lsp/handler.rs — Request handlers
    - [ ] handle_initialize() — respond with server capabilities
    - [ ] handle_did_open() — track opened documents
    - [ ] handle_did_change() — re-validate on content changes
    - [ ] handle_shutdown() — cleanup
  - [ ] Wire up Command::Lsp in src/main.rs
  - [ ] Create tests/lsp_basic.rs — acceptance tests
    - [ ] initialize request/response
    - [ ] document open/change/close tracking
  - [ ] All tests pass

- [ ] LSP Phase 2: Real-time Validation
  - [ ] Create src/lsp/diagnostics.rs — Convert Issue → LSP Diagnostic
    - [ ] Map error codes to severity
    - [ ] Include error message, code, help text
  - [ ] Integrate with existing checker::run()
  - [ ] Validate on textDocument/didOpen and didChange
  - [ ] Publish diagnostics for all errors (E001, E002, E003, etc.)
  - [ ] Create tests/lsp_diagnostics.rs — validation tests
    - [ ] E001 errors generate correct diagnostics
    - [ ] Multiple errors aggregated
    - [ ] Clean files produce no diagnostics
  - [ ] Test with real .agile.md files

- [ ] LSP Phase 3: IDE Integration
  - [ ] Document VS Code setup (.vscode/settings.json)
  - [ ] Document Vim/Neovim setup (init.lua example)
  - [ ] Add LSP section to README.md
  - [ ] Provide troubleshooting guide

- [ ] LSP Phase 4: Enhanced Features (Optional)
  - [ ] textDocument/hover — show property definitions
  - [ ] textDocument/codeAction — quick fixes for common errors
  - [ ] textDocument/completion — suggest properties, users, groups
  - [ ] File diagnostics on save with `agile check --fix`


## More CLI features
