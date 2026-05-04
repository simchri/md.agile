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
- **Orphaned indented task** — A task has leading whitespace but no parent task on the preceding line (usually due to a blank line separating parent and child). Indicates the task should be un-indented to top-level.
- **Wrong indentation** — A task's indentation does not match a valid subtask level.
  - quickfix: Auto-correct indentation to match nesting depth
- **Wrong Body Indentation** — A task body line is incorrectly indented 
  - quickfix: Auto-correct indentation 
- **Missing Space After Box** — A task line is missing a space after the status box (e.g. `- [ ]task` instead of `- [ ] task`)
  - quickfix: Insert missing space
- **Incomplete Parent** — A parent task is marked done but has incomplete children (no quick fix)
- **Invalid Box Style** — The status box contains an unrecognised character (e.g. `- [o] task`, `- [] task`). Valid boxes are `[ ]`, `[x]`, and `[-]`.
  - quickfix: Replace the invalid box with `[ ]`
- **Uppercase X** — The status box uses an uppercase X (e.g. `- [X] task`). Use lowercase `[x]` instead.
  - quickfix: Replace `[X]` with `[x]`

## GUI

After installation launch in the directory with the md.agile.tasks and toml file: 
```
mdagile-gui
```
Then connect to localhost 8080 in a browser (in browser search bar type `http://127.0.0.1:8080/`).


## Project Philosophy

See [MANIFESTO.md](MANIFESTO.md)
