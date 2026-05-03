# md.agile

Plain-text, version-controlled task management for developers.

**Status**: Early prototype. Basic parsing and file discovery work. Core checker and LSP server are operational but cover only the first validation rule.

For upcoming features, see [README.vision.md](README.vision.md)


Simple animated board

<img width="800" height="402" alt="animated_board" src="https://github.com/user-attachments/assets/476af559-b4fb-4558-8f09-258a97f1e176" />

Formatting rules enforced via language server with quick fixes!

<img width="800" height="768" alt="quickfix" src="https://github.com/user-attachments/assets/aaecaa6e-a735-4ae4-98b3-90da54981000" />


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

A minimal LSP server that runs on stdin/stdout. Advertises text document sync (FULL mode), publishes real-time diagnostics as you edit, and offers quickfix code actions for fixable issues.

**Supported diagnostics:**
- **E001: Orphaned indented task** — A task has leading whitespace but no parent task on the preceding line (usually due to a blank line separating parent and child). Indicates the task should be un-indented to top-level.
- **E002: Wrong indentation** — A task's indentation does not match a valid subtask level. Expected indentation is `depth * 2` spaces (2 spaces per nesting level).
  - quickfix: Auto-correct indentation to match nesting depth

### File Priority & Task Ordering

Files are ordered alphabetically by their path relative to the project root (directory components first, then filename). This establishes global task priority:
- `tasks/50_current/001.agile.md` outranks `tasks/60_backlog/001.agile.md`
- Within each file, top-to-bottom order determines priority

The first incomplete task across all files is the highest-priority work.



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
