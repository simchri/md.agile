# LSP Task Navigation

`agilels` exposes a set of "jump to task" actions that let you move directly
between tasks across your whole workspace.
This document explains what's available and how to wire it up in your
editor.

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

## Neovim

Requires `nvim-lspconfig` and the `agilels` binary on `PATH`. See
[INSTALL.md](../INSTALL.md#nvim) for the base server registration
(`filetypes`, `root_dir`, etc.).

The standard requests need no extra config — bind them like any other LSP
keymap (commonly done in an `LspAttach` autocmd):

```lua
vim.keymap.set("n", "gD", vim.lsp.buf.declaration, { buffer = bufnr })
vim.keymap.set("n", "gi", vim.lsp.buf.implementation, { buffer = bufnr })
-- or "gI" (uppercase) - advantage: "i" always stays bound to "insert mode"
```

The custom commands need to be dispatched via
`vim.lsp.buf.execute_command`, passing the current buffer URI and (for
`next*`/`previous*`) the 0-based cursor line:

```lua
local function mdagile_jump(command, with_line)
  return function()
    local bufnr = vim.api.nvim_get_current_buf()
    local uri = vim.uri_from_bufnr(bufnr)
    local args = { uri }
    if with_line then
      local line = vim.api.nvim_win_get_cursor(0)[1] - 1 -- 0-based
      table.insert(args, line)
    end
    vim.lsp.buf.execute_command({ command = command, arguments = args })
  end
end

vim.keymap.set("n", "<leader>gtp", mdagile_jump("mdagile.jump.highestPriorityOpen", false)) -- "go task prio"
vim.keymap.set("n", "<leader>gtm", mdagile_jump("mdagile.jump.highestPriorityMy", false)) -- "go task mine"
vim.keymap.set("n", "<leader>gtn", mdagile_jump("mdagile.jump.nextOpen", true))-- "go task next"
vim.keymap.set("n", "<leader>gtp", mdagile_jump("mdagile.jump.previousOpen", true))-- "go task previous"
vim.keymap.set("n", "<leader>gtN", mdagile_jump("mdagile.jump.nextMy", true)) -- "go task next (mine)"
vim.keymap.set("n", "<leader>gtP", mdagile_jump("mdagile.jump.previousMy", true)) -- "go task previous (mine)"
-- additional direct shortcuts (no leader) -- for your most important actions
vim.keymap.set("n", "<C-A-j>", mdagile_jump("mdagile.jump.nextMy", false)) 
vim.keymap.set("n", "<C-A-k>", mdagile_jump("mdagile.jump.nextMy", false)) 
```

Place these inside the same `LspAttach` autocmd (or `on_attach`) used for
your other buffer-local LSP keymaps, guarded to only apply when the
attaching client is `agilels`.

## VS Code

There's no dedicated `mdagile` VS Code extension yet, so `agilels` must be
wired up through a generic LSP client extension, e.g.
[`vscode-generic-lsp-client`](https://marketplace.visualstudio.com/items?itemName=arcanis.vscode-zipfs)-style
setup or a small custom extension. At minimum, the client needs to:

- Register `agilels` as the language server for `*.agile.md` files.
- Start it with `cmd: "agilels"`, `args: []`, and the workspace root as
  `rootUri`.

Once connected:

- `Shift+F12` (**Go to Declaration**) and `Ctrl+F12` (**Go to
  Implementation**) work immediately — these are core VS Code commands
  that any language client wires up automatically once the server
  advertises `declarationProvider`/`implementationProvider`.
- The custom `mdagile.jump.*` commands are **not** visible in the Command
  Palette by default; a client extension must contribute them explicitly
  (`contributes.commands` in `package.json`) and invoke them with
  `vscode.commands.executeCommand("workbench.action.executeLspCommand", ...)`
  or, more directly, by having the extension call
  `vscode.commands.executeCommand("<command>", currentUri, currentLine)` after
  registering the command through `vscode-languageclient`'s
  `sendRequest("workspace/executeCommand", { command, arguments })`. Bind
  the resulting extension commands to keybindings via `keybindings.json`,
  e.g.:

```json
{
  "key": "ctrl+alt+n",
  "command": "mdagile.jump.nextOpen"
}
```

(the exact command IDs depend on how your chosen client extension
re-exposes them — check its docs for the naming convention it uses to proxy
`workspace/executeCommand`).
