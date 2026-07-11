# Md.Agile

Plain-text, version-controlled task management for developers.
Your tasks live forever in a simple text file, version-controlled directly alongside your code (ideally in the same repository) – not a web app!

> This project is >> under construction <<  - more features to come. (c.f. [README.vision.md](README.vision.md))

**tasks.md:**

```md
- [ ] a task - this is the task-title
  Some more info on this task - this is the task-body
  Both inside and outside of tasks, you can just use normal markdown syntax
  - [ ] a subtask 
    more details for this subtask go here.
  - [x] this subtask is done
```

Tasks follow a specific syntax. You will receive immediate feedback in your text editor if you make a mistake (via Language Server). Use the language server's auto-fix feature for an ergonomic experience. Use the CLI tool to add strict checks as pre-commit hooks or in your pipeline – your task list is always consistent. Everything is designed with a "command line first" approach: text files and CLI tools are the primary interface. Graphical client for convenient "board view" (Currently only a viewer, no edits possible).

Simple animated board

<img width="400" height="auto" alt="animated_board" src="https://github.com/user-attachments/assets/476af559-b4fb-4558-8f09-258a97f1e176" />

Formatting rules enforced via language server with quick fixes!

<img width="400" height="auto" alt="quickfix" src="https://github.com/user-attachments/assets/aaecaa6e-a735-4ae4-98b3-90da54981000" />

## Installation

See [INSTALL.md](INSTALL.md) for building/installing the CLI, language server, and editor setup (currently source-only via `cargo install`).

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
Parses all `*.agile.md` files and reports validation issues. Exits with status 1 if any issues found, 0 if clean. See [doc/checks.md](doc/checks.md) for the full list of checks, and [doc/config.md](doc/config.md) for the `mdagile.toml` reference.

### Language Server: `agilels`

A minimal LSP server that offers real-time diagnostics as you edit, and offers quickfix code actions for fixable issues. Runs the same checks as `agile check` — see [doc/checks.md](doc/checks.md).

## GUI

After installation launch in the directory with the md.agile.tasks and toml file: 
```
mdagile-gui
```
Then connect to localhost 8080 in a browser (in browser search bar type `http://127.0.0.1:8080/`).


## Project Philosophy

See [MANIFESTO.md](MANIFESTO.md). Term definitions (Marker, Property, Assignment, Special Marker, etc.) are in [GLOSSARY.md](GLOSSARY.md).
