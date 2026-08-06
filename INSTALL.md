
# Installation

Project is not yet configured for easy convenient installation on all platforms. If you are a developer, go for it! (Non-technical users, expect some difficulties!)

## Option 1 - Build from Source - Docker

Preferred installation method for developers, experienced linux users

Supported:
- Debian based Linux distros (Debian, Ubuntu etc.) 

Prerequisites: 
- Project sources available (e.g. project cloned)
- [Docker](https://docs.docker.com/engine/install/) 
- [Make](https://www.gnu.org/software/make/) (Preinstalled on must linux systems)

Just run
```
make install
```
The `install` target will create packages (e.g. *.deb files) for your OS and install all components (cli, gui, lsp). If you want to install only some components, instead run `make package`, then manually install only the wanted packages. Package files are in `./target`.  

Uninstall:
- Remove executables via your package manger, e.g. debian based distro: `sudo apt remove "mdagile*"`
- Optionally; Clean docker images & containers (c.f. Docker documentation)
- Optionally; remove this project directory

## Option 2 - Build from Source - On host

Supported:
- Any Linux distro supported by the rust toolchain

Prerequisites: 
- Project sources available (e.g. project cloned)
- [Rust](https://rust-lang.org/tools/install/)

Installation of cli and language server:
```
cargo install --path crates/cli
```
Installation of board viewer:
Currently not easily installable. Use development workflow for testing.

Uninstall:
- Remove executables via cargo `cargo uninstall mdagile`
- Optionally; remove this project directory
- Optionally; Clean rust toolchain cache / remove rust toolchain

## Language Server Integration

The steps above only make the executable files available on the host system. If you want to use the language server (the "IDE integration") for mdagile, you have to configure your IDE/editor accordingly. 

### Nvim

Perform installation as described above (any option) and check that `agilels` binary is on your path (`whereis agilels`). Then add the config below.

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
