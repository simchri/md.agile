# Architecture

Rough architecture for the `mdagile` / `agile` CLI tool.

---

## Processing Pipeline

Every command follows the same five-step pipeline:

```
mdagile.toml  ──►  Config
*.agile.md    ──►  Parser  ──►  AST  ──►  Checker  ──►  Rules  ──►  Command output
```

1. **Config** — load and validate `mdagile.toml` (properties, users, groups)
2. **Discover** — find all `*.agile.md` files from project root, sort by filename
3. **Parse** — convert each file into a typed AST (`TaskFile`)
4. **Aggregate** — merge all `TaskFile`s into a single ordered task list
5. **Check / Query** — run rules against the aggregated list; commands read the result

The checker and rules are always run — even for read commands — so that the tool can
warn about invalid state.

---

## Module Layout

```
src/
├── main.rs            # CLI entry point; dispatches subcommands
├── config.rs          # mdagile.toml loading and Config types
├── discovery.rs       # file-finding logic
├── parser/            # *.agile.md → AST
│   ├── mod.rs
│   ├── lexer.rs       # tokenise lines into Tokens
│   └── ast.rs         # TaskFile, Task, Marker, Milestone … types
├── checker/           # validate AST against rules
│   ├── mod.rs         # orchestrates rule passes; collects Diagnostics
│   └── diagnostic.rs  # Diagnostic { span, message, severity }
├── rules/             # all business logic (pure functions, fully testable)
│   ├── mod.rs
│   ├── completion.rs  # can a task be marked done?
│   ├── ordering.rs    # ordered-subtask constraints
│   ├── properties.rs  # required subtasks, short form, nested/branch props
│   ├── assignments.rs # @user / @group resolution and enforcement
│   ├── milestones.rs  # milestone reachability
│   └── eta.rs         # task weights and ETA calculation
├── ui/                # TUI components (business-logic-free)
│   ├── mod.rs
│   ├── task_list.rs   # interactive task viewer (agile)
│   └── task_new.rs    # new-task creation mask (agile task new)
└── lsp/               # Language Server Protocol
    ├── mod.rs
    ├── server.rs      # LSP message loop
    └── capabilities.rs# diagnostics, completions, code-actions (autofix)
```

---

## Key Data Types

### Config (from `mdagile.toml`)

```rust
struct Config {
    properties: HashMap<String, PropertyConfig>,
    users:      HashMap<String, UserConfig>,
    groups:     HashMap<String, GroupConfig>,
}

struct PropertyConfig {
    subtasks:             Vec<String>,          // may include "#nested_prop" references
    subtasks_allow_cancel: Vec<bool>,
    short:                Option<String>,       // short-form alias
    branches:             HashMap<String, BranchConfig>,
    neighbortasks:        Vec<String>,
}

struct BranchConfig {
    neighbortasks: Vec<String>,
}

struct UserConfig  { full_name: String, email: String }
struct GroupConfig { members: Vec<String> }
```

### AST (from parser)

```rust
struct TaskFile {
    path:  PathBuf,
    items: Vec<FileItem>,       // ordered: tasks, milestones, other content
}

enum FileItem {
    Task(Task),
    Milestone(Milestone),
    Directive(FileDirective),   // e.g. #MDAGILE.file.mandatory_property=…
    OtherContent(String),
}

struct Task {
    status:   TaskStatus,       // Todo | Done | Cancelled
    title:    String,           // raw text after "- [ ] "
    body:     String,           // free-text lines before next blank
    subtasks: Vec<Task>,
    markers:  Vec<Marker>,      // parsed from title
    order:    Option<u32>,      // Some(n) if prefixed "n. "
}

enum TaskStatus { Todo, Done, Cancelled }

enum Marker {
    Property(String),           // #name  (resolved against Config)
    PropertyShort(String),      // short-form alias
    BranchProperty { name: String, branch: Option<String> }, // #review... / #review:passed
    Assignment(String),         // @name
    Special(SpecialMarker),
}

enum SpecialMarker { Opt, Milestone, MdAgile }

struct Milestone { name: String }
```

### Diagnostics

```rust
struct Diagnostic {
    file:     PathBuf,
    line:     usize,
    severity: Severity,   // Error | Warning | Info
    code:     &'static str,
    message:  String,
}
```

---

## CLI Commands

| Invocation         | Action                                              |
|--------------------|-----------------------------------------------------|
| `agile`            | Interactive TUI task viewer (next tasks)            |
| `agile check`      | Run checker; print diagnostics; exit non-zero on error |
| `agile task new`   | TUI mask to create a new task (top or bottom of backlog) |
| `agile when`       | ETA to each milestone (uses task weights + velocity) |
| `agile fix`        | Auto-fix common issues (add missing required subtasks, etc.) |

`agile check` is the only command intended for use in pre-commit hooks and CI.

---

## LSP

The LSP server wraps the same pipeline:

- **Diagnostics** — re-runs checker on file save; publishes `textDocument/publishDiagnostics`
- **Completions** — suggests `#property` and `@user` tokens from `Config`
- **Code actions** — "autofix" actions: add missing required subtasks, resolve short-form markers

The LSP does not share state with the CLI at runtime; both are thin shells over the same
`parser`, `checker`, and `rules` crates.

---

## Identity Resolution

`agile` resolves the current user by:
1. CLI flag `--user`
2. Env var `MDAGILE_USER`
3. `git config user.email` (matched against `[Users]` in `mdagile.toml`)

Assignment enforcement is convention, not access control (see MANIFESTO.md).

---

## File Ordering and Priority

Multiple `*.agile.md` files are sorted **alphabetically by filename** (not path). The
order of tasks within this merged list is the global priority order — topmost = highest
priority. The `tasks.agile.md` file at the project root is a conventional "main" backlog
but has no special status beyond its sort position.
