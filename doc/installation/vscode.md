# VS Code Setup

There's no dedicated `mdagile` VS Code extension yet, so `agilels` must be
wired up through a generic LSP client extension, e.g.
[`vscode-generic-lsp-client`](https://marketplace.visualstudio.com/items?itemName=arcanis.vscode-zipfs)-style
setup or a small custom extension. Make sure `agilels` is on `PATH` first
(see [install.md](install.md)). At minimum, the client needs to:

- Register `agilels` as the language server for `*.agile.md` files.
- Start it with `cmd: "agilels"`, `args: []`, and the workspace root as
  `rootUri`.

## Jump-to-task navigation

Once connected, `agilels`'s "jump to task" navigation actions become
available — see [doc/usage/lsp_task_navigation.md](../usage/lsp_task_navigation.md)
for what each action does.

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
