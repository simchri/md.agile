# Architecture

Rough architecture for the `mdagile` / `agile` CLI tool.

---

## Crate Stack

| Concern | Crate | Notes |
|---|---|---|
| Markdown structure | `pulldown-cmark` | GFM task lists, nesting, escaping — ~500 MB/s |
| File discovery | `ignore` | ripgrep's walker: parallel, `.gitignore`-aware |
| CLI args | `clap` | |
| TUI | `ratatui` + `crossterm` | |
| Fuzzy matching | `nucleo` | Used by Helix; faster than skim |
| LSP protocol | `lsp-server` | Sync stdio scaffold (from rust-analyzer team) |
| Config | `toml` + `serde` | |
| Git history (ETA) | `git2` | Walk commits for velocity; fallback: shell `git log` |

---

## Processing Pipeline

Every command follows the same pipeline:

```
mdagile.toml  ──►  Config
*.agile.md    ──►  Parser  ──►  AST  ──►  Checker  ──►  Rules  ──►  Command output
```

1. **Config** — load and validate `mdagile.toml` (properties, users, groups)
2. **Discover** — `ignore::WalkBuilder` finds all `*.agile.md` files; sort by filename
3. **Parse** — `pulldown-cmark` handles list structure + nesting; a small inline scanner
   extracts mdagile-specific tokens (`#marker`, `@user`, `1.` order prefix) from item text
4. **Aggregate** — merge all `TaskFile`s into a single ordered task list
5. **Check / Query** — run rules against the aggregated list; commands read the result

The checker is always run — even for read-only commands — so the tool can warn about
invalid state.

### Parser: two layers

`pulldown-cmark` is not replaced with a custom lexer. It owns everything structural:
indentation, nesting, checkbox state, escaped characters (`\#`, `\@`). The custom layer
is a ~50-line inline scanner that runs on the *text content* of each list item to extract
`Marker`s and the optional order prefix. This keeps parsing correct and fast while
staying minimal.

---

## Module Layout

```
src/
├── main.rs            # CLI entry point; dispatches subcommands
├── config.rs          # mdagile.toml loading and Config types (serde + toml)
├── discovery.rs       # thin wrapper around ignore::WalkBuilder
├── parser/            # *.agile.md → AST
│   ├── mod.rs         # drives pulldown-cmark; feeds items through inline_scan
│   ├── inline_scan.rs # ~50-line scanner: extracts Markers and order prefix from text
│   └── ast.rs         # TaskFile, Task, Marker, Milestone … types
├── checker/           # validate AST against Config + rules
│   ├── mod.rs         # orchestrates rule passes; collects Diagnostics
│   └── diagnostic.rs  # Diagnostic { file, line, severity, code, message }
├── rules.rs           # all business logic (pure functions, fully testable)
│                      # split into submodules only if it exceeds ~300 lines
├── eta.rs             # task weight calculation + milestone ETA (reads git2 velocity)
├── ui/                # TUI components — business-logic-free
│   ├── mod.rs
│   ├── task_list.rs   # interactive viewer (agile): ratatui + nucleo fuzzy filter
│   └── task_new.rs    # new-task creation mask (agile task new): ratatui-textarea
└── lsp/               # Language Server Protocol
    ├── mod.rs
    ├── server.rs      # lsp-server message loop + file-cache (see below)
    └── capabilities.rs# diagnostics, completions, code-actions (autofix)
```

---

---

## CLI Commands

| Invocation         | Action |
|--------------------|--------|
| `agile`            | Interactive TUI task viewer; plain-text output when stdout is not a TTY (pipe-friendly) |
| `agile check`      | Run checker; print diagnostics; exit non-zero on error — use in pre-commit / CI |
| `agile task new`   | TUI mask to create a new task (top or bottom of backlog) |
| `agile when`       | ETA to each milestone (task weights + git velocity) |
| `agile fix`        | Auto-fix common issues: add missing required subtasks, resolve short-form markers |

### TTY / pipe composability

`agile` detects whether stdout is a TTY. In a TTY it renders the `ratatui` interactive
viewer with `nucleo` fuzzy search. When piped it prints plain text, so users can compose
with external tools they already have:

```sh
agile | fzf
agile | grep "#bug"
```

---

## LSP

The LSP server wraps the same pipeline. It uses `lsp-server` for JSON-RPC over stdio —
no async runtime needed since the client drives all I/O timing.

- **Diagnostics** — re-check on `textDocument/didChange`; publish `textDocument/publishDiagnostics`
- **Completions** — suggest `#property` and `@user` tokens from `Config`
- **Code actions** — autofix: add missing required subtasks, resolve branch-property outcomes

### Incremental parsing

Re-parsing the entire project on every keystroke is not acceptable. The LSP server
maintains a file cache:

```
HashMap<PathBuf, (FileHash, TaskFile)>
```

On `didChange`, only the modified file is re-parsed. The checker re-runs over the cached
ASTs of all files. A change to `mdagile.toml` invalidates the full cache.

---

## ETA and Velocity

`agile when` computes ETA as:

```
remaining_weight / velocity  →  date
```

- **Remaining weight** — sum of `task_weight(t)` for all incomplete tasks before each milestone
- **Velocity** — average weight completed per day, derived from git history

Velocity is read from the git log via `git2`: walk commits, diff task files, count weight
of tasks that transitioned to `[x]`. This avoids shelling out and works in any environment
where a `.git` directory is present.

Task weight: a task itself is weight 1; a subtask at nesting level `n` has weight `1/n`.

---

## Identity Resolution

`agile` resolves the current user by:
1. CLI flag `--user <key>`
2. Env var `MDAGILE_USER`
3. `git config user.email` matched against `[Users]` in `mdagile.toml`

Assignment enforcement is convention, not access control (see MANIFESTO.md).

---

## File Ordering and Priority

`ignore::WalkBuilder` finds all `*.agile.md` files. They are sorted **alphabetically by
filename** (basename only, not full path). The merged task list order is the global
priority — topmost = highest priority. `tasks.agile.md` at the project root has no
special status beyond its sort position.
