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
- [x] testing for first lsp feature
- [x] refactor and understand


## First Language Server Protocol (LSP) Features

- [x] LSP Phase 1: Core Foundation (Hello World)
  Entry point: `agile lsp` (stdin/stdout JSON-RPC)
  - [x] Create src/lsp/protocol.rs — LSP message types (serde)
    - [x] InitializeRequest/Response
    - [x] DidOpenTextDocument / DidChangeTextDocument notifications
    - [x] PublishDiagnosticsNotification
    - [x] JsonRpc message wrapper
  - [x] Create src/lsp/mod.rs — Main server loop
    - [x] Read JSON-RPC from stdin
    - [x] Dispatch to handlers
    - [x] Write responses to stdout
  - [x] Create src/lsp/handler.rs — Request handlers
    - [x] handle_initialize() — respond with server capabilities
    - [x] handle_did_open() — track opened documents
    - [x] handle_did_change() — re-validate on content changes
    - [x] handle_shutdown() — cleanup
  - [x] Wire up Command::Lsp in src/main.rs
  - [x] Create tests/lsp_basic.rs — acceptance tests
    - [x] initialize request/response
    - [x] document open/change/close tracking
  - [x] All tests pass

- [x] LSP Phase 2: Real-time Validation
  - [x] Create src/lsp/diagnostics.rs — Convert Issue → LSP Diagnostic
    - [x] Map error codes to severity
    - [x] Include error message, code, help text
  - [x] Integrate with existing checker::run()
  - [x] Validate on textDocument/didOpen and didChange
  - [x] Publish diagnostics for all errors
  - [x] Create tests/lsp_diagnostics.rs — validation tests
    - [x] All error types generate correct diagnostics
    - [x] Multiple errors aggregated
    - [x] Clean files produce no diagnostics
  - [x] Test with real .agile.md files

## More basic Syntax Checks and Quick Fixes
- [x] wrongly indented task description
  task description starts exactly here, at the same location as the "[ ]".
  This applies to every line of the task description.
  - [x] agile check
    Some more dummy subtask content to test the visual appearance of the indentation. Lorem ipsum dolor sit amet,
    consectetur adipiscing elit. Donec a diam lectus. Sed sit amet ipsum mauris.
    Maecenas congue ligula ac quam viverra nec consectetur ante hendrerit. 
    Donec et mollis dolor. Praesent et diam eget libero egestas mattis sit amet vitae augue. 
    Nam tincidunt congue enim, ut porta lorem lacinia consectetur. Donec ut libero sed arcu vehicula ultricies a non tortor. 
    Lorem ipsum dolor sit amet, consectetur adipiscing elit. Aenean ut gravida lorem.
  - [x] agilels hint
  - [x] agilels quickfix: fix indentation
- [x] missing space between task box and beginning of task title
  - [x] agile check
  - [x] agilels hint
  - [x] agilels quickfix: Add space

## Subtasks are Mandatory
- [x] detect parent task incorrectly marked as done, even though subtasks are not done
  - [x] agile check
  - [x] agilels hint
    Note: For now- no quickfix. Users should decide if subtasks can be deleted, cancelled or marked done

## GUI prototype
- [x] overview of suitable GUI frameworks
  suitable framework must:
  - compile natively to windows and linux. Mobile not required atm
  - support nice animations for "moving post-its over a canvas"
  - be lightweight and performant
  - popular and well supported
  - styling with css
- [x] GUI framework selected: Dioxus
- [x] Hello world GUI app running
- [x] prototype runs: Empty canvas, divided into three rows, narrow top row, wide middle row, narrow bottom row. Separated by simple black lines.
- [x] load title of next task onto post-it
- [x] next task dynamically updated on file changes
- [x] make all information of task available on the frontend side of the UI 
  - [x] render subtasks as well in the post it
    just clip if not enough space. keep post-it side constant)
- [x] place post-it along top-left → bottom-right diagonal based on subtask completion progress (done in bottom-right corner)

- [x] task post-it can be opened by clicking
  - [x] task modal then takes up most of the available screen and overlays all other elements
  - [x] modal is made scrollable
  - [x] all content can be viewed
- [x] load in backlog tasks along the top row
  load the ten next tasks in the backlog
  - [x] backlog task post-its are a bit smaller than task in progress
- [x] handle all task post-its the same way in the code. Not different items
  This implies that multiple post its can be in the middle part of the view, moving over the screen- that's what we want!
  - [-] assign new css classes, when at least one sub-task is checked (larger box)
  - [x] all post its are conceptually handled the same way
- [x] display the last ten tasks that were done along the bottom row
  - [x] done task post-its are a bit smaller than task in progress
- [ ] z-ordering: Last changed task is moved to front

##  Properties and first Settings

- [ ] Property & Assignment validation
  Detect undefined #property markers and @user/@group assignments
  - [ ] Read mdagile.toml config in checker; pass config to rules
  - [ ] Detect undefined #property markers in tasks
    - [ ] Implement fuzzy matching to suggest close matches (typo detection)
    - [ ] Test with common typos: #Feature, #feat, etc.
  - [ ] Detect undefined @user and @group assignments
    - [ ] Suggest close matches for misspelled names
    - [ ] Handle OR connections: @markus or @josh
  - [ ] Update error formatter for new error codes

- [ ] Missing required subtasks
  Detect when a task has a property (e.g. #feature) but lacks required subtasks
  - [ ] Match quoted subtasks in tasks against property definitions from mdagile.toml
  - [ ] Handle multiple properties on same task
  - [ ] Handle nested properties (e.g., #feature that includes #review)
  - [ ] Provide helpful error with list of missing subtasks
  - [ ] Tests: single property, multiple properties, nested properties

- [ ] Invalid order markers
  Detect duplicate order numbers, gaps, or malformed ordering syntax
  - [ ] Validate no duplicate ranks (e.g., two "2." markers)
  - [ ] Detect gaps in sequence (1, 3, skip 2)
  - [ ] Ensure ordering is only at same sibling level
  - [ ] Tests for various invalid orderings

- [ ] Data integrity: Incomplete parent tasks warning
  Warn when a parent marked done [x] still has [ ] children
  - [ ] this is an error (exit 1)?
  - [ ] Consider: add --strict flag to promote warnings to errors


## More CLI features

- [ ] create a global overview of the planned CLI structure as some markdown file, with a tree-like view
  - [ ] list of subcommands and their functions
  - [ ] most important flags to each subcommand
  - [ ] let human review and adjust the overview

## Build More CLI features
...

## LSP documentation

- [ ] LSP Phase 3: IDE Integration
  - [ ] Document VS Code setup (.vscode/settings.json)
  - [ ] Document Vim/Neovim setup (init.lua example)
  - [ ] Add LSP section to README.md
  - [ ] Provide troubleshooting guide

## More LSP features

- [ ] LSP Phase 4: Enhanced Features (Optional)
  - [ ] textDocument/hover — show property definitions
  - [x] textDocument/codeAction — quick fixes for common errors
  - [ ] textDocument/completion — suggest properties, users, groups
  - [ ] File diagnostics on save with `agile check --fix`


