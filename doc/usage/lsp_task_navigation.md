# LSP Task Navigation

`agilels` exposes a set of "jump to task" actions that let you move directly
between tasks across your whole workspace.
This document explains what's available and how it behaves; see
[Editor setup](#editor-setup) below for how to wire it up in your editor.

## General information

There are two ways the actions are exposed, both served by the same
`agilels` binary:

### Standard LSP requests

| Request | Typical default keybinding | Behaviour |
|---|---|---|
| `textDocument/declaration` | `gD` (Vim/Neovim), `Shift+F12` (VS Code) | Jump to the highest-priority open task in the workspace (falls back to the current file if no project root is known). |
| `textDocument/implementation` | `gi` (Vim/Neovim), `Ctrl+F12` (VS Code) | Jump to *your* highest-priority open task — the first open task eligible for your identity, as resolved from git config and `mdagile.toml`/`.mdagile.toml`. |

These map onto existing, editor-native keybindings, so they typically work
out of the box once the language server is active in your editor — no custom
keymap/config needed.

### Custom `workspace/executeCommand` commands

For finer-grained, cursor-relative navigation, `agilels` advertises six
custom commands. These have no default keybinding in any editor — you must
bind them yourself.

| Command | Arguments | Behaviour |
|---|---|---|
| `mdagile.jump.highestPriorityOpen` | `[currentUri]` | Same as `textDocument/declaration`. |
| `mdagile.jump.highestPriorityMy` | `[currentUri]` | Same as `textDocument/implementation`. |
| `mdagile.jump.nextOpen` | `[currentUri, currentLine]` | Next open task after the cursor line. |
| `mdagile.jump.previousOpen` | `[currentUri, currentLine]` | Previous open task before the cursor line. |
| `mdagile.jump.nextMy` | `[currentUri, currentLine]` | Next open task after the cursor line, eligible for you. |
| `mdagile.jump.previousMy` | `[currentUri, currentLine]` | Previous open task before the cursor line, eligible for you. |

`currentUri` is the requesting document's URI (string); `currentLine` is the
0-based cursor line, required only by the `next*`/`previous*` commands. Each
command moves the cursor via a `window/showDocument` request (taking editor
focus) and returns a boolean indicating whether a target task was found — it
does **not** return a `Location`, so it can't be bound as a regular
goto-definition-style action; it must be invoked as a workspace command.

All actions prefer the live in-editor buffer content (including unsaved
edits) over on-disk file content when resolving targets, and search across
every `*.agile.md` file under the project root, not just the currently open
one.

## Editor setup

See [doc/installation/neovim.md](../installation/neovim.md) or
[doc/installation/vscode.md](../installation/vscode.md) for how to wire
these actions up in your editor.
