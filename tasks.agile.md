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
- [x] refactor: each TaskView lives in its own signal (vector of 50 pre-allocated slots; backend tasks beyond the limit are dropped)
- [x] z-ordering: Last changed task is moved to front

## Additional (Simple) Syntax & Validations
- [x] no empty boxes "[]"
- [x] no boxes filled with anything else
- [x] refactor: "ParsingIssues" to Partial Item, to avoid adding more and more fields here
- [x] Quickfix for [X] --> "[x]"
- [x] Quickfix for other invalid boxes --> "[ ]"
- [x] indication for when quickfix available


##  Properties and first Settings

- [x] Create config module: parse property definitions from mdagile.toml
  Adds `src/config/` with `Config::from_str` and `Config::load`. Reads `[Properties.<name>]`
  sections using the `toml` crate; validates the file is well-formed TOML.

- [x] BUG: board broken
- [x] BUG: language server not found by nvim (some config stuff?) 
  --> cany needed update. Since we added the tty flag as default in devenv, cany needs to do --no-tty

- [x] post mortem: add some non-regression checks to the GUI
  Seems like we are kind of limited here, unit test, ok, otherwise best idea seems to be
  - [x] add some UTs
  - [-] smoke test, that checks if warnings are issued.
    Will be slow and require playwright to find any actual issues (server dormant with no clients connected) -> don't do

- [x] some GUI fixes - for the fun of it!
  - [x] tasks properly sized and positioned
    - [x] scale size of task post-its (done and backlog) with viewport size. height should match exactly the height of the "separator" sections
    - [x] post-its positions in backlog and progress exactly aligned with the separator sections
    - [x] .. then make the post-its just a little bit smaller (like 5px or so)

- [x] GUI style improvements, red, green coloring, sepia tint, monospace
  - [x] pastel green text color for done tasks
  - [x] pastel red color for cancelled tasks
  - [x] sepia tint of the board
  - [x] use a monospace font
  - [x] Problem: text on some tasks does not fit on the card. Don't render text all the way to the bottom of the task, but have a fade-out shadow gradually hide it (fading into the color of the task card). Make tasks with a lot of overflowing text more visually appealing

- [x] GUI: don't update (get tasks) while a task is maximized

## GUI 1.0
- [x] test case / script:
  - [x] launches a gui instance via dx pointing to a fixture directory
  - [x] simulates tasks with subtasks being created (backlog)
  - [x] lorem ipsum text in tasks
  - [x] simulates tasks being marked done, so that they continuously progress over the board
  - [x] multiple tasks are in progress at the same time
  - [x] at least one task "overtakes" other tasks
  - [x] plays the whole thing on repeat
- [x] Repel/spread: overlapping in-progress cards push each other apart
  Cards that would overlap negotiate their position along (or perpendicular to)
  the diagonal so none fully obscure another. Progress still determines the
  rough position; the spread is only the local adjustment needed to avoid overlap.
  - [x] refactor the current code, to easily accomodate setting both x and y postion for each card explicitly.
  - [x] the normal target position is the current diagonal postion
- [x] rework repel to be fully 2D (also spread left-right, not just perp to axis)
- [x] repel fixes
- [x] experiment with other designs for backlog and done sections
  Style with gradients

- [x] some installability improvements

- [ ] Property & Assignment validation
  Detect undefined #paaroperty markers and @user/@group assignments
  - [x] Read mdagile.toml config in checker; pass config to rules
  - [x] Detect undefined '#property' markers in tasks
    - [x] basic detection and errors "#foo" '#bar'
      - [x] agile check
        - [x] bug: diagnostic column indication is wrong (seems to be always at 0)
          - [x] reproduce in a test case (failing)
          - [x] fix (make test pass)
      - [x] language server
        - [x] quickfix to add a respective toml entry 
        - [x] dynamically update the diagnostics, after e.g. quickfix was applied
  - [ ] BUG: Somewhere between d705c83df0697ad826d7f85490b37b9d849fbddb and 9ab03a50 the property validation in both agile check and language server broke completely
    - [ ] add better tests that can actually detect failures like this
  - [x] "go to definition" to toml entry
    - [ ] Implement fuzzy matching to suggest close matches (typo detection) #OPT
    - [ ] Test with common typos: '#Feature', '#feat', etc. #OPT
  - [ ] Detect undefined @user and @group assignments
    - [ ] Suggest close matches for misspelled names
    - [ ] Handle OR connections: '@markus' or '@josh'
  - [ ] Update error formatter for new error codes

- [ ] Missing required subtasks
  Detect when a task has a property (e.g. '#feature') but lacks required subtasks
  - [ ] Match quoted subtasks in tasks against property definitions from mdagile.toml
  - [ ] Handle multiple properties on same task
  - [ ] Handle nested properties (e.g., '#feature' that includes '#review')
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
- [x] introduce a logging library for the CLI crate and replace raw `eprintln!` calls with structured log calls (uses `tracing`, controlled by `AGILE_LOG`)


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


## GUI 2.0
- [ ] find tasks by search string - list with done state, rank, full name
- [ ] mark tasks done from CLI
  - [ ] tasks
    - [ ] by rank
    - [ ] directly by search term
  - [ ] subtasks
    - [ ] by rank (34.1.3 etc, where subtasks get a nested rank)
    - [ ] directly by search term

### Writing back to file from GUI
- [ ] Mark task as done from GUI
