
# Installation

Project is not yet configured for easy convenient installation on all platforms. If you are a developer, go for it! (Non-technical users, expect some difficulties!)

Prereqs: 
- Rust toolchain installed
- Project sources available (e.g. project cloned)

Install the cli and language server with cargo - from project dir:
```
cargo install --path crates/cli
```

Alternatively, `./scripts/install.sh` (see below) builds and installs the
cli too, alongside the GUI.

## Board viewer (GUI)

The GUI additionally needs the `dx` CLI (Dioxus CLI) and the
`wasm32-unknown-unknown` Rust target, since it bundles a small web frontend
alongside its server. `./scripts/install.sh` (this runs directly on your
machine — no Docker involved) has three independent, individually
selectable modes:

- `--toolchain` — installs the `wasm32-unknown-unknown` target and
  `dioxus-cli` (assumes Rust/cargo itself is already installed).
- `--cli` — builds and installs the `agile` and `agilels` binaries.
- `--gui` — bundles the web assets and installs the `agilegui` binary.

Run with no flags to do all three:

```
./scripts/install.sh
```

Or run just one mode, e.g. to (re-)install prerequisites, or refresh the
GUI after pulling new changes:

```
./scripts/install.sh --toolchain
./scripts/install.sh --gui
```

The `--gui` mode bundles the web assets, bakes them into a single
self-contained `agilegui` binary (no separate `public/` folder to manage),
and installs it to `~/.local/bin/agilegui`. Run it with:

```
MDAGILE_WORKDIR=/path/to/your/project agilegui
```

Then open the printed URL (usually `http://127.0.0.1:8080/`) in your browser.


## Language Server
### Nvim

After installation, ensure the `agilels` binary is on your path. Then add the config below.

Nvim config example with lazy pkg manager (if you use a different package manager, adjust as needed):

```lua
#~/.config/nvim/lua/plugins/lang-mdagile.lua

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
