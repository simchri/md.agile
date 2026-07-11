# Md.Agile

Plain-text, version-controlled task management for developers.
Your tasks live forever in a simple text file, version-controlled directly alongside your code (ideally in the same repository) – not a web app!

> This project is >> under construction <<  - more features to come. (c.f. [README.vision.md](README.vision.md))

**tasks.md:**

```md
- [ ] a task - this is the task-title
  Some more info on this task - this is the task-body
  Both inside and outside of tasks, you can just use normal markdown syntax
  - [ ] a subtask @markus
    more details for this subtask go here.
  - [x] this subtask is done
```

Tasks follow a specific syntax. You will receive immediate feedback in your text editor if you make a mistake (via Language Server). Use the language server's auto-fix feature for an ergonomic experience. Use the CLI tool to add strict checks as pre-commit hooks or in your pipeline – your task list is always consistent. Everything is designed with a "command line first" approach: text files and CLI tools are the primary interface. Graphical client for convenient "board view" (Currently only a viewer, no edits possible).

Simple animated board

<img width="400" height="auto" alt="animated_board" src="https://github.com/user-attachments/assets/476af559-b4fb-4558-8f09-258a97f1e176" />

Formatting rules enforced via language server with quick fixes!

<img width="400" height="auto" alt="quickfix" src="https://github.com/user-attachments/assets/aaecaa6e-a735-4ae4-98b3-90da54981000" />

## Syntax

A task that is "done" is marked with a lowercase `x`; `-` marks a task as cancelled. Subtasks must be indented two spaces per level, directly under their parent (no blank line in between) — a blank line always starts a new top-level task:

```md
- [ ] a task
  - [ ] a subtask
  - [x] this subtask is done
  - [-] this subtask is cancelled

- [ ] another top-level task
```

Other content (headings, prose, etc.) is ignored, so you can freely mix notes into the same file. See [doc/checks.md](doc/checks.md) for the full list of formatting/marker rules `agile check` enforces.

### Multiple Files

Task files must be named `<something>.agile.md` and can live anywhere under the project root; every match is picked up automatically (`agile list files` shows them in priority order). Files are ordered alphabetically by path relative to the project root (directories first, then filename) — this determines cross-file task priority, so e.g. `tasks/50_current/001.agile.md` outranks `tasks/60_backlog/001.agile.md`.

### Optional Subtasks

By default, all subtasks are mandatory — a parent task can't be marked done while any required subtask is still open. Mark a subtask optional with `#OPT`:

```md
- [ ] a task
  - [ ] #OPT some optional subtask
```

### Properties

Declare a `#property` in `mdagile.toml`, then tag tasks with it:

```toml
[Properties.feature]
subtasks = ["PO review", "dev implementation", "test"]
```

```md
- [ ] #feature: add item to basket
  - [ ] "PO review"
  - [ ] "dev implementation"
  - [ ] "test"
```

Required subtasks (declared via `subtasks`) must be present as quoted (`"..."`) child subtasks, in any order. Using an undeclared `#property` is a validation error. Tasks can carry multiple properties at once, and a property's own required subtasks can themselves carry other properties (nested). Required subtasks can optionally be cancellable instead of completable — see `subtasks_allow_cancel` in [doc/config.md](doc/config.md).

### Assignment / Completion Validation

Assign a task to a person or group with `@name`:

```md
- [ ] implement feature X @markus
  - [ ] review @QA
```

`agile check` flags a task marked done by someone who isn't an assignee (or a member of an assigned group) — see [doc/assignment-validation.md](doc/assignment-validation.md). This is automation, not access control — it's trivially bypassable, and is meant as a gentle nudge (see [MANIFESTO.md](MANIFESTO.md)).

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
