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

- [x] #feature: Property & Assignment validation #foo
  Detect undefined #property markers and @user/@group assignments
  Note: property markers are only recognized in the task title (#here #they #are #ignored)
  - [x] Read mdagile.toml config in checker; pass config to rules
  - [x] Detect undefined '#property' markers in tasks
    - [x] basic detection and errors "#foo" '#bar'
      - [x] agile check
        - [x] bug: diagnostic column indication is wrong (seems to be always at 0)
          - [x] reproduce in a test case (failing)
          - [x] fix (make test pass) #OPT
      - [x] language server
        - [x] quickfix to add a respective toml entry
        - [x] dynamically update the diagnostics, after e.g. quickfix was applied
  - [x] "go to definition" to toml entry
    - [x] go to def in lsp
    - [x] Implement fuzzy matching to suggest close matches (typo detection)
    - [x] Test with common typos: '#Feature', '#feat', etc.
  - [x] Detect undefined "@user" and "@group" assignments
    - [x] basic implementation
    - [x] bug: ("@bob") ("#someundefproperty") asdf"#anotherundefprop" -> Done
    - [x] Suggest close matches for misspelled names
  - [-] Update error formatter for new error codes
  - [x] GoTo for "@assignments"
    - [x] basic implementation @alice
    - [x] BUG: go to definition does not work if the assignment or feature marker is not separated by whitespace e.g. some"#feature" hernameis@alice -- the used assumption is that there will always be whitespace is wrong
      - [x] fix
      - [x] refactor: The detection logic of markers and properties in files should be centralized to avoid bugs like above
  - [x] "(feature) validation by programmer"
  - [x] "(feature) implementation"
  - [x] "bar"
  - [x] "baz"

- [x] #feature syntax highlighting for #OPT
  - [x] "(feature) validation by programmer"
  - [x] "(feature) implementation"
- [x] #feature syntax highlighting for '#MILESTONES'
  - [x] basic implementation
  - [x] double check details of #MILESTONE handling. AI did something weird here. Handled it a bit like a normal property
    - [x] '#Milestone' --> undef property. OK
    - [x] #MILESTONE not a highlighted as keyword here
  - [x] #MDAGILE double check #MDAGILE special marker handling - should this be highlighted as keyword here?
    No! config keys still to be implemented - re-visit later. N.B. MDAGILE tag currently never highlighted, but that's ok since keys are not implemented anyways
  - [x] "(feature) validation by programmer"
  - [x] "(feature) implementation"

- [x] #feature: syntax highlighting for assignments: asdf@alice
  - [x] "(feature) validation by programmer"
  - [x] "(feature) implementation"

- [x] syntax highlighting for '#properties'

- [x] introduce a logging library for the CLI crate and replace raw `eprintln!` calls with structured log calls (uses `tracing`, controlled by `AGILE_LOG`)

#MILESTONE: Some milestone

- [x] Missing required subtasks
  Detect when a task has a property (e.g. '#feature') but lacks required subtasks
  - [x] Match quoted subtasks in tasks against property definitions from mdagile.toml
  - [x] Handle multiple properties on same task
  - [x] Handle nested properties (e.g., '#feature' that includes '#review')
  - [x] Provide helpful error with list of missing subtasks
  - [x] Tests: single property, multiple properties, nested properties

- [x] E010 acceptance test
  End-to-end CLI test for E010 in tests/check.rs (spawns real binary with a temp project).
  All other rules (E001, E008…) already have coverage at this level; E010 needs the same.
  - [x] missing required subtasks → exit 1 + E010 in stdout
  - [x] all required subtasks present → exit 0

- [x] #feature LSP quickfix for E010 (insert missing required subtasks)
  The IssueData::MissingRequiredSubtasks { missing } payload is already in place.
  The vision explicitly calls out autofix: "use the autofix feature of your text editor
  to quickly add the required subtasks."
  - [x] Add lsp/quickfix/missing_required_subtasks.rs builder
  - [x] Insert each missing quoted subtask as a new child line after the last existing child (or after the task line if no children)
  - [x] Register in the REGISTRY in lsp/quickfix/mod.rs
  - [x] Tests for the quickfix builder
  - [x] validation
  - [x] "(feature) validation by programmer"
  - [x] "(feature) implementation"

- [x] #feature Check that detects subtasks completely surrounded by quotes (- [ ] "some subtask"), but which are NOT required subtasks
  This syntax is reserved for required subtasks (by #properties) and should not be usable otherwise
  - [x] "(feature) implementation"
  - [x] "(feature) validation by programmer"

- [x] #foo Allow cancelling required subtasks (subtasks_allow_cancel)
  When a property defines `subtasks_allow_cancel`, individual required subtasks may be cancelled without error
  - [x] Extend `PropertyConfig` with `subtasks_allow_cancel: Vec<bool>` (parallel to `subtasks`)
  - [x] Parse `subtasks_allow_cancel` array from `[Properties.X]` in mdagile.toml
  - [x] Update E010 rule: treat a cancelled required subtask as satisfied only if its allow_cancel flag is true, otherwise report error
  - [x] Tests: cancel allowed, cancel not allowed, mixed array
  - [x] "bar"
  - [x] "baz"

- [x] assignment / completion validation: Ensure only assigned people can mark a task as done.
  This check shall be available whenever we are working in a git repo and an identity for the current user can be retreived.
  This identity is checked agains definitions in the mdagile.toml file. E.g. We could indicate a list of mail addresses there. If the current git identity matches one of the addresses of the user, the user is considerd authenticated
  This feature is not secure in any way, but only aims to provide some gentle nudging towards doing the right thing.
  The check is run using changes in the working copy vs. last committed change. users can overpower this check by just committing anyway
  - [x] Config: extend `UserConfig` with `git_emails: Vec<String>` (identity match) and `git_names: Vec<String>` (fallback match against `git config user.name`)
  - [x] Config: extend `GroupConfig` with `members: Vec<String>` referencing `[Users.X]` keys
  - [x] Resolve current git identity by shelling out to `git config user.email` / `git config user.name` (no new git library dependency)
    - [x] Match email against any user's `git_emails`; fall back to matching `user.name` against `git_names` if no email match
    - [x] If not in a git repo, or no identity resolves, silently skip the whole check (no diagnostic)
  - [x] Retrieve the HEAD version of a file via `git show HEAD:<relpath>`; handle untracked/new files (no HEAD version exists)
  - [x] Detect done-transitions by parsing both the HEAD and working-copy versions and matching tasks/subtasks by title/content (not line number, which is fragile across unrelated edits)
    - [x] A task with no match in HEAD (new file, or title changed alongside status) that is already `[x]` in the working copy is also treated as a transition to check
  - [x] For each detected transition to `[x]`: gather `user`/`group` markers on that task
    - [x] No assignment marker present → skip (anyone may complete an unassigned task)
    - [x] Authorized identities = directly assigned `user`s + members of any assigned `group`
    - [x] Authorized if current identity matches ANY of the above; multiple assignees only need one match
  - [x] New rule + error code (next available, e.g. E013) "UnauthorizedCompletion" — reported as an error (exit 1), consistent with all other rules
  - [x] Integrate into `agile check` (this rule needs git + HEAD file content, unlike the pure `&[FileItem]`-only rules in `rules::check_all` — needs its own orchestration path)
  - [x] Integrate into LSP live diagnostics (re-run the same git-based comparison on `didOpen`/`didChange`)
  - [x] Tests
    - [x] authorized direct assignee completes task → no issue
    - [x] unauthorized user completes directly-assigned task → error
    - [x] group-assigned task completed by a group member → no issue
    - [x] group-assigned task completed by a non-member → error
    - [x] unassigned task completed by anyone → no issue
    - [x] multiple assignees, current identity matches one → no issue
    - [x] identity resolved via email match
    - [x] identity resolved via `git_names` fallback when email doesn't match
    - [x] outside a git repo → check silently skipped
    - [x] no git identity configured → check silently skipped
    - [x] new/untracked file with a task created already `[x]` and misassigned → flagged
    - [x] task title changed alongside status change (no HEAD match) → still flagged if misassigned
    - [x] LSP diagnostics test covering this rule

- [x] validation fix: If current user does not appear in mdagile.toml, user should be considered "unauthorized" -> error
  currently, this is not the case, c.f. also comment in md.agile/crates/cli/src/checker/mod.rs:29:6
  - [x] Introduce a `ResolvedIdentity` enum (`Known(String)` / `Unrecognized`) replacing the current `Option<String>` used internally between `resolve_repo_identity` and `rules::unauthorized_completion`
  - [x] `Unrecognized` (identity determined but doesn't match any `[Users.X]`) → always unauthorized for any assigned task; unassigned tasks are unaffected
  - [x] `None` (no identity determinable at all: not a git repo, or `git config user.email`/`user.name` both empty, and no `--as` override) → still silently skip the whole check, unchanged
  - [x] Update/rename existing tests that assumed "identity not in config" silently skips (e.g. `check_authorization_skipped_when_identity_unresolvable`) to expect unauthorized instead

- [x] assignment / completion validation: CI/CD flexibility (identity override + configurable diff base)
  Motivating use case: running `agile check` in a CI/CD pipeline against a PR, where the git identity of the CI runner isn't the PR author's identity, and the code is already fully committed (no working-copy diff against HEAD to detect).
  - [x] New `agile check --as <user-key>` flag: overrides identity resolution entirely with a literal `[Users.X]` key (no email/`git_names` fallback matching)
    - [x] `--as` value not found in `config.users` → `Unrecognized` (unauthorized for assigned tasks), consistent with the general "unknown identity = unauthorized" rule above, not a distinct CLI usage error
    - [x] When `--as` is given, it takes priority over `git config user.email`/`user.name` (the ambient git identity is not consulted at all)
  - [x] New `agile check --base <git-ref>` flag: overrides the hard-coded `HEAD` used for the "old" side of the diff (e.g. `origin/main`, a SHA, or a ref computed by the CI script via `git merge-base`)
    - [x] The "new" side remains the on-disk working directory (unchanged) — covers the common CI case where the PR's code is already checked out; no support for enumerating files from an arbitrary "new" ref (would require `git ls-tree` instead of walking disk)
    - [x] Invalid/non-existent `--base` ref → hard CLI error (distinct failure mode from an authorization violation), not a silent skip
  - [x] Generalize `git::head_file_content` (or add a new function) to fetch file content at an arbitrary ref, validating the ref itself exists (to distinguish "bad ref" from "file doesn't exist at this ref", the latter being a legitimate new/untracked-file case)
  - [x] Tests
    - [x] `--as` overriding a locally-configured git identity
    - [x] `--as` with an unrecognized user key → unauthorized error
    - [x] `--base` comparing two committed refs (no working-copy changes) still detects a `[ ] -> [x]` transition
    - [x] `--base` with an invalid ref → CLI error, not silently skipped
    - [x] combination of `--as` + `--base` together (the CI use case end-to-end)

- [x] validate mdagile.toml config file. `agile check` should return with an error in case of unknown config keys

- [x] validate that any "members" list in mdagile.toml only contains actually defined users.

- [x] rename the mdagile.toml property "emails" to "git_emails" (consistent with git_names)

- [x] bugfix: assignment / completion validation (E013) matched old vs. new tasks/subtasks by bare title text alone, which collides whenever two different tasks have same-titled subtasks (e.g. `property`-required subtasks reuse the same literal title, like "bar"/"baz" in mdagile.toml, across every task carrying that property). This could cause both false negatives (a genuine unauthorized completion goes unflagged) and false positives. Found during a critical code review of recent E013 work.
  - [x] Reproduced with a regression test: two same-titled subtasks under different parent tasks, one with a genuine new transition, one already-done and unchanged
  - [x] Fixed by matching on the full ancestor-title path (root task down to the node) instead of bare title

- [x] when tasks live in a git repo, but the current users identity can not be determined, when runnning `agile check` issue a warning on the terminal (assignment validation not possible etc. ladi ladi da..) . (for lsp, continue just silently skip validation checks)
  - [x] also warn when the project isn't in a git repo at all (not just "in a repo but no identity"), suppressible per-project via `[General] warn_when_not_a_git_repo = false` in mdagile.toml, since some projects intentionally don't use git

## Ordering Tasks
- [x] Invalid order markers — ordered tasks enforcement rules (E014/E015). Scope was narrowed to match what README.vision.md actually specifies (duplicate ranks are forbidden; nothing is said about requiring contiguous numbering, so "gaps" and "malformed syntax" detection were dropped as unwritten/invented spec — discussed and confirmed with the user):
  - [x] E014: reject duplicate ranks among siblings (e.g., two "2." markers)
  - [x] E015: prevent marking a ranked task done while any lower-ranked sibling is still incomplete (not done and not cancelled)
  - [x] Ensure both checks are scoped to the same sibling list only (a rank has no meaning across different parents or nesting levels)
  - [x] Tests: unit tests per rule + acceptance tests (`e014.rs`, `e015.rs`)

## Misc.
- [x] Escaping marker characters: a backslash before a marker character should make it literal text instead of a property/assignment marker (see README.vision.md "Basic Syntax", `not_a_property` example). There's no backslash-escape handling in the parser at all currently — the marker character is always parsed as a marker, regardless of any preceding backslash.

- [x] Checks on ordered subtasks should apply also to mandatory subtasks from properties. The escaping / application logic of the check must be revised

## Code review fixes
- [x] `find_task_files` was not actually respecting `.gitignore`/`.ignore` despite its doc comment claiming so (`WalkBuilder` had `.ignore(false)` and `.git_ignore(false)` set). Fixed by removing those overrides so the walker uses its git-aware defaults. Added acceptance test `list_files_respects_gitignore` in `crates/cli/tests/acceptance/list/files.rs`.
- [x] `parse_milestone_name` accepted the MILESTONE tag glued directly to an alphanumeric suffix (e.g. `MILESTONEfoo` after the leading hash, silently became milestone "foo") with no boundary check. Fixed by requiring a non-alphanumeric character right after the tag. Added unit tests `milestone_tag_glued_to_alphanumeric_suffix_is_not_recognized` and `milestone_tag_with_punctuation_boundary_is_still_recognized` in `crates/cli/src/parser/tests.rs`.
- [x] E013 unauthorized-completion matching collapsed old/new nodes sharing the same ancestor-title path into a single HashMap entry, so duplicate sibling titles (legal, nothing forbids them) could cause an already-done sibling to be false-positively re-flagged. Fixed by matching same-path occurrences positionally (nth old occurrence to nth new occurrence in document order) instead of overwriting a single map entry. Added regression test `duplicate_sibling_titles_are_matched_positionally_not_collapsed` in `crates/cli/src/rules/unauthorized_completion/tests.rs`.
- [x] The parser silently accepted tasks/subtasks with no title text at all (e.g. `- [ ] ` with nothing after the box, or a line consisting only of markers), producing blank entries with zero diagnostics. Introduced a new `ParsingIssue::EmptyTitle` and error code E016 "Empty title", wired into a new `rules::empty_title` check (`check_all`). Added unit tests in `crates/cli/src/rules/empty_title/tests.rs` and an acceptance test file `crates/cli/tests/acceptance/checks/e016.rs`; documented in `doc/checks.md`.
- [x] `resolve_identity_user` matched a git email/name against every configured user by iterating a HashMap, whose order is randomized per process. If two `[Users.X]` entries in mdagile.toml accidentally shared the same email or git name, which user was resolved (for E013 attribution) varied unpredictably across runs, with no config-time error to catch the mistake. Fixed by adding a deterministic `find_duplicate_identity` validation in `Config::from_str` that rejects such config with a new `ConfigError::DuplicateIdentity` before any HashMap lookup can become ambiguous. Added unit tests in `crates/cli/src/config/tests.rs`.
- [x] The LSP silently swallowed any config-load failure (invalid TOML, conflicting mdagile.toml/.mdagile.toml, property/group/identity validation errors) via `.unwrap_or_default()`, falling back to an empty config with zero indication to the user, unlike the CLI which hard-fails with a clear error. This meant every config-dependent check (E007-E013) could silently go dark in the editor. Fixed by tracking config-error state on the LSP `Backend` and, on error, popping a `window/showMessage` notification (once per distinct error, not spammed every keystroke) plus a synthetic diagnostic on the document being validated. Added acceptance tests in `crates/cli/tests/acceptance/lsp/config_error.rs`; documented in `doc/config.md`.
- [x] Follow-up to the LSP config-error fix above: while the config-error diagnostic was shown, the LSP still ran the full rule set (`checker::run`) against the empty fallback `Config`, so every property/assignment marker in the open document was also reported as spuriously undefined (E008/E009) alongside the real config error. Fixed by splitting `rules::check_all` into `rules::check_config_independent` (structural checks only) + the four config-dependent rules, and having the LSP call only the config-independent subset (and skip the E013 authorization check entirely) whenever config loading failed. Added unit tests in `crates/cli/src/rules/tests.rs` and an acceptance test in `crates/cli/tests/acceptance/lsp/config_error.rs`.
- [x] Seven lint rules (`invalid_box`, `uppercase_x`, `missing_space_after_box`, `incomplete_parent`, `wrong_body_indent`, `unrequired_quoted_subtask`, `missing_required_subtasks`) each hand-rolled their own recursive tree walk over `Task`/`Subtask` instead of reusing `rules::for_each_node`, which previously only exposed `(markers, location, indent)` and so could only serve `undefined_property`/`undefined_assignment`. Introduced a `NodeRef<'a>` view (over `Task`/`Subtask`) exposing `location`, `indent`, `status`, `markers`, `body`, `children`, and `parsing_issues`, and changed `for_each_node` to yield it. Rewrote all 7 rules to use the shared traversal, removing ~140 lines of duplicated recursion; `wrong_indentation` was left as-is since it needs depth-tracking and the task-only `preceded_by_blank` flag, which aren't generic node concerns.
- [x] Config discovery (locating `mdagile.toml`/`.mdagile.toml`) was implemented three times with slightly different behavior: `Config::load` and the LSP's `find_config_file`/`config_file_for_path` each independently repeated the file-name existence-check loop, and only `Config::load` detected the "both files present" conflict. Extracted a shared `config::CONFIG_FILE_NAMES` constant plus `config::find_config_file_in`/`config::find_config_file_upwards` helpers; `Config::load` and both LSP call sites now delegate to them instead of re-declaring the file-name list, and the redundant `Backend::find_config_file` wrapper was removed.

- [x] #bug check hint / name "ranked task completed out of order" --> "ordered task completed ..." (rank is priority rank, order is order of subtasks. Update the glossary too, to clarify this globally)

- [x] #bug `agile check` --  "ranked (ordered) task completed out of order" the column indicator always points to column 0. Could be nicer: Point to the ordering number instead
- [x] check other similar checks - is this marker placed logically?


## More CLI features

- [x] create a global overview of the planned CLI structure as some markdown file, with a tree-like view
  - [x] list of subcommands and their functions
  - [x] all flags to each subcommand
  - [x] clearly mark what is already implemented / what's missing
  - [x] let human review and adjust the overview

### ctd cli commands
- [x] remove unused flag: `agile task next --next N`, instead:
- [x] command: `agile task next 3` show the next 3 tasks
- [x] command: `agile task next 1.1` show the first subtask of the next task
- [x] command: `agile task next 2.2` show the first sub-subtask of the second task etc.
- [x] command: `agile task done 2.2` mark the respective task as done, unless this violates any rules on completion of tasks (e.g. subtasks not complete) - then show the error message instead. Efficient implementation, avoid checking the whole project.
- [x] command: `agile task next --mine` show the next task eligible.
  Eligbility --> same rules as for assignment / completion validation 

- [x] mark tasks done from CLI
  - [x] tasks
    - [x] by rank
    - [x] directly by search term
  - [x] subtasks
    - [x] by rank (34.1.3 etc, where subtasks get a nested rank)


### cli refactor:
- [x] "rename agile task next N.M" to "agile task show N.M"

- [x] re-structure the command "agile list" as subcommand to "task"/"tasks", i.e. `agile tasks list ..`
- [x] re-structure "agile list files" to "agile file" (synonym: "files")

- [x] range support for "agile list tasks", syntax `agile tasks list START:END` ranges apply to the top-level only, but show tasks with subtasks

## First GUI write actions
- [x] Mark task as done from GUI
- [x] --kiosk flag to disable any "writing" operations

- [x] GUI: Errors as snackbars - an example for dx snackbars can be found in the project /data/ws/buckett - use that example to implement snack bars here
- [x] inverse to "mark done": mark tasks "not done" from the cli!
- [x] GUI: Un-mark subtasks and tasks as done (mark as "todo")

## ETA
- [ ] Milestones: ETA / time estimation. 
  The MILESTONE special marker is parsed (divides tasks into milestone groups) and syntax-highlighted, but there's no `agile when` command, 
  no average-time-per-task estimation, and no task-weight system (subtask weight = 1/nesting-level, used only for ETA math) implemented at all.

### Short Forms
- [ ] Property short forms: a `short` key in a `[Properties.X]` config entry (see README.vision.md "Property Short Forms"), allowing a task to carry a lightweight marker (subtasks not required yet) while still blocking completion until the full property replaces it. Not present in the config schema at all yet.

## GUI 2.0
- [ ] backlog view in GUI
- [ ] create tasks from GUI
- [ ] graveyard view
- [ ] menu
  - [ ] switch / select projects

- [ ] improved installation & launch procedure for GUI

## Neighbor Tasks / Branch Properties / Workflows
- [ ] Neighbor Tasks: a `neighbortasks` config key on a `[Properties.X]` entry (see README.vision.md "Neighbor Tasks"), requiring a specific sibling task to exist alongside the property-carrying task/subtask. Not present in the config schema; no corresponding validation rule.
- [ ] Branch Properties (see README.vision.md "Branch Properties"): the pending/resolved outcome syntax is already recognized by the parser (`PropertyForm::BranchPending`/`BranchResolved`), but nothing acts on it yet — no rule requires resolving to a defined outcome before marking the task done, and outcome-specific `neighbortasks`/`subtasks` (e.g. a `[Properties.review.passed]` sub-table) aren't read from config at all.

## LSP documentation
- [ ] LSP Phase 3: IDE Integration
  - [ ] Document VS Code setup (.vscode/settings.json)
  - [ ] Document Vim/Neovim setup (init.lua example)
  - [ ] Add LSP section to README.md
  - [ ] Provide troubleshooting guide

## Archiving, multiple files
- [ ] File structure for large projects & archiving:
  - [ ] `agile init --large` — scaffold the `tasks/00_archive`, `50_current`, `60_backlog`, `80_inbox` directory structure
  - [ ] `agile archive` — move any file in `50_current`/`60_backlog` containing only completed/cancelled tasks into `00_archive`, prefixed with today's date
  - [ ] `[Archive]` config section (`archive_path`, `current_path`, `backlog_path`, `inbox_path`)

## File level Props
- [ ] Required properties per file: a file-level directive (see README.vision.md "Required Properties", `MDAGILE.file.mandatory_property=<name>`) declaring a property mandatory for every task in that file. The MDAGILE special marker is currently only recognized/highlighted as a generic token — the directive's `file.mandatory_property=X` value isn't parsed, and there's no rule enforcing it or auto-adding the property to new tasks.

## More CLI features
- [ ] Apply existing quick fixes via `agile fix` on the command line
- [ ] `agile task undone` cannot reopen an already fully-done top-level task (only a done subtask under a still-open parent) — its address only counts still-incomplete top-level tasks, same as `agile task done`, so a done top-level task is never reachable by number. Needs a dedicated way to reopen a whole completed top-level task (e.g. a separate addressing scheme/command), if that's ever wanted.

## More LSP features
- [ ] LSP Phase 4: Enhanced Features (Optional)
  - [ ] textDocument/hover — show property definitions
    - [ ] properties: Add optional help texts / descriptions to properties that can be shown on hover
    - [ ] idem "@assignments" relevant, e.g. for groups
  - [ ] textDocument/completion — suggest properties, users, groups

## Events
- [ ] Think about "events" as a separate /parallel concept to tasks. Use: Appear on the board as a sort of blocker, indicating that tasks are not worked on (because the people are "blocked")
  - [ ] formulate "vision"
