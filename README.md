# md.agile

Plain-text, version-controlled task management for developers.

**Status**: Early prototype. Basic parsing and file discovery work. Core checker and LSP server are operational but cover only the first validation rule.

For upcoming features, see [README.vision.md](README.vision.md)

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
- **E003: Wrong body indentation** — Task description lines don't match the expected indentation level.
  - quickfix: Auto-correct indentation to match task level
- **E004: Incomplete parent** — A task marked done still has incomplete children (unless they are marked `#OPT`).
- **E005: Missing space after status box** — No space between `[ ]` and task title.
  - quickfix: Add space after status box

### GUI Prototype: Agile Board

A Tauri-based visual task board prototype with a three-row canvas layout (Inbox, Current, Backlog). Drag-and-drop post-its between rows, change task status with buttons.

**Features:**
- Canvas-based UI with draggable post-it notes
- Three rows: Inbox (top, narrow), Current (middle, wide), Backlog (bottom, narrow)
- Color-coded by status: todo (yellow), doing (blue), done (green)
- Smooth animations and hover effects
- Real-time task position and status sync via Tauri backend

**Build & Run:**

From the `gui/` directory:
```bash
npm install                 # Install JavaScript dependencies (once)
npm run dev               # Start development server
npm run build             # Build for production
```

Or in Docker:
```bash
docker compose run dev-container-no-gpu
cd gui
npm install
npm run dev
```

See [`gui/README.md`](gui/README.md) for more details.

### File Priority & Task Ordering

Files are ordered alphabetically by their path relative to the project root (directory components first, then filename). This establishes global task priority:
- `tasks/50_current/001.agile.md` outranks `tasks/60_backlog/001.agile.md`
- Within each file, top-to-bottom order determines priority

The first incomplete task across all files is the highest-priority work.



## Building & Testing

**CLI & LSP (Rust only):**
Requires Rust nightly (pinned in `docker/Dockerfile`). Edition 2024.

```bash
cargo build                      # Build CLI and LSP
cargo run                        # Run CLI
./target/debug/agilels          # Run LSP (runs on stdin/stdout)

cargo test                       # Run full test suite
cargo test --lib -- test_name   # Run single test
```

**GUI Prototype (JavaScript + Rust):**
Requires Node.js 16+ and npm, plus Tauri system dependencies (automatically included in the Docker image).

```bash
cd gui
npm install
npm run dev                      # Development with hot reload
npm run build                    # Production build
```

## Project Philosophy

See [MANIFESTO.md](MANIFESTO.md) for the design rationale behind the text-based, git-integrated approach.
