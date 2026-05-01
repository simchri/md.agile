# md.agile

Plain-text, version-controlled task management for developers.

**Status**: Early prototype. Basic parsing and file discovery work. Core checker and LSP server are operational but cover only the first validation rule.

## What Works

### File Discovery & Parsing
- Recursively finds `*.agile.md` files anywhere under the project root
- Parses task syntax: `- [status] title` where status is ` ` (todo), `x` (done), or `-` (cancelled)
- Supports arbitrary nesting of subtasks (indented by 2 spaces per level)
- Records file path and line number for every task

### CLI Tool: `agile`

#### Default: Open Next Task
```bash
agile
```
Opens the highest-priority incomplete task in your `$VISUAL` or `$EDITOR`. Jumps to the correct line for vim, nvim, nano, emacs, and VS Code.

#### List Tasks
```bash
agile list              # All active tasks (default)
agile list --all       # All tasks including done and cancelled
agile list -n 5        # First 5 active tasks
agile list --last 3    # Last 3 active tasks
```

#### List Files
```bash
agile list files       # Show all task files in priority order
```

#### Get Next Task
```bash
agile task next        # Print the next incomplete task (same as `agile` with no editor)
```

#### Validate Files
```bash
agile check
```
Parses all `*.agile.md` files and reports validation issues. Exits with status 1 if any issues found, 0 if clean.

### Language Server: `agilels`

A minimal LSP server that runs on stdin/stdout. Advertises text document sync (FULL mode) and publishes real-time diagnostics as you edit.

**Supported diagnostics:**
- **E001: Orphaned indented task** — A task has leading whitespace but no parent task on the preceding line (usually due to a blank line separating parent and child). The diagnostic highlights the leading whitespace that should be removed.

Example: VS Code or Neovim with LSP client configured to use `agilels` will show warnings for orphaned tasks as you type.

### File Priority & Task Ordering

Files are ordered alphabetically by their path relative to the project root (directory components first, then filename). This establishes global task priority:
- `tasks/50_current/001.agile.md` outranks `tasks/60_backlog/001.agile.md`
- Within each file, top-to-bottom order determines priority

The first incomplete task across all files is the highest-priority work.

## What Does NOT Work Yet

The vision (see [README.vision.md](README.vision.md)) includes many features not yet implemented:

- **Markers & Properties**: The `#property` and `@assignment` syntax is recognized by the parser but not validated or enforced
- **Rules**: Only indentation is checked; no other validation rules exist
- **Configuration**: `mdagile.toml` support is scaffolded but not functional
- **Users & Groups**: Assignment logic (`@user`, `@group`) is not implemented
- **Mandatory/Optional Subtasks**: Properties that define required subtasks (`#OPT`, `#feature` with subtasks)
- **Milestones & ETA**: No milestone support or time estimation
- **LSP Features**: No auto-fix suggestions, no symbol completion
- **Advanced Workflows**: Ordered tasks, branch properties, neighbor tasks not implemented
- **Project Structure**: No support for large-project file layout or archiving

## Building & Testing

Requires Rust nightly (pinned in `docker/Dockerfile`). Edition 2024.

```bash
cargo build                      # Build CLI and LSP
cargo run                        # Run CLI
./target/debug/agilels          # Run LSP (runs on stdin/stdout)

cargo test                       # Run full test suite
cargo test --lib -- test_name   # Run single test
```

## Project Philosophy

See [MANIFESTO.md](MANIFESTO.md) for the design rationale behind the text-based, git-integrated approach.
