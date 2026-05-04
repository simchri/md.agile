
# Installation

Project is not yet configured for easy convenient installation on all platforms. If you are a developer, go for it! (Non-technical users, expect some difficulties!)

Prereqs: 
- Rust toolchain installed
- Project sources available (e.g. project cloned)

Install the cli and language server with cargo - from project dir:
```
cargo install --path crates/cli
```

board viewer:
Currently not easily installable. Use development workflow for testing.


## Language Server
### Nvim

After installation, ensure the `agilels` binary is on your path. Then add the config below.

Nvim config example with lazy pkg manager (if you use a different package manager, adjust as needed):

```lua
#.config/nvim/lua/plugins/lang-mdagile.lua

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
