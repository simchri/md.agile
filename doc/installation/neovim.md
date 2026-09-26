# Neovim Setup

Requires `nvim-lspconfig` and the `agilels` binary on `PATH` (see
[install.md](install.md)).

## Server registration

Check that `agilels` binary is on your path (`whereis agilels`). Then add the
config below.

Nvim config example with lazy pkg manager (if you use a different package
manager, adjust as needed):

```lua
-- ~/.config/nvim/lua/plugins/lang-mdagile.lua

-- register the file extension ".agile.md" as both "markdown" and "agile" file type
-- this ensures both your usual markdown features and the ls are active
vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
  pattern = "*.agile.md",
  callback = function()
    vim.bo.filetype = "markdown.agile" -- combined file type: This is both "markdown" and "agile"
  end,
})

-- this block is necessary, because agilels is not in the standard lspconfig database
require("lspconfig.configs").agilels = {
  default_config = {
    cmd = { "agilels" },
    filetypes = { "markdown.agile" },
    root_dir = function(fname)
      return vim.fs.dirname(vim.fs.find({ ".git" }, { upward = true, path = fname })[1])
    end,
    settings = {},
  },
}

return {
  {
    "neovim/nvim-lspconfig",
    opts = {
      servers = {
        agilels = {
          cmd = {
            "agilels",
          },
        },
      },
    },
  },
}
```

## Jump-to-task navigation keymaps

Once registered, `agilels` also exposes "jump to task" navigation actions —
see [doc/usage/lsp_task_navigation.md](../usage/lsp_task_navigation.md) for
what each action does.

The standard requests need no extra config — bind them like any other LSP
keymap (commonly done in an `LspAttach` autocmd):

```lua
vim.keymap.set("n", "gD", vim.lsp.buf.declaration, { buffer = bufnr })
vim.keymap.set("n", "gi", vim.lsp.buf.implementation, { buffer = bufnr })
-- or "gI" (uppercase) - advantage: "i" always stays bound to "insert mode"
```

The custom commands need to be dispatched via `workspace/executeCommand`,
passing the current buffer URI and (for `next*`/`previous*`) the 0-based
cursor line. **Don't** use `vim.lsp.buf.execute_command()` for this: it
broadcasts the request to *every* LSP client attached to the buffer, and
other clients that also advertise `executeCommandProvider` (e.g. GitHub
Copilot's language server) will receive the `mdagile.jump.*` command too,
correctly reject it as unknown, and surface a noisy error notification on
every jump. Look up the `agilels` client explicitly and call
`client.request()` on it directly instead:

```lua
local function mdagile_jump(command, with_line)
  return function()
    local bufnr = vim.api.nvim_get_current_buf()
    -- vim.lsp.buf.execute_command() broadcasts to every client attached to
    -- the buffer (e.g. copilot also advertises executeCommandProvider),
    -- causing spurious "Unknown command" errors from other clients. Target
    -- the agilels client directly instead.
    local client = vim.lsp.get_clients({ bufnr = bufnr, name = "agilels" })[1]
    if not client then
      vim.notify("mdagile: no agilels client attached to this buffer", vim.log.levels.WARN)
      return
    end
    local uri = vim.uri_from_bufnr(bufnr)
    local args = { uri }
    if with_line then
      local line = vim.api.nvim_win_get_cursor(0)[1] - 1 -- 0-based
      table.insert(args, line)
    end
    client.request("workspace/executeCommand", { command = command, arguments = args }, nil, bufnr)
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
