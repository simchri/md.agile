## Building & Testing

Run complete test suite for project: (On host):
```
make test
```
The makefile handles a docker dev environment. If you want more fine grained control, keep reading.

### Devenv

The dev instructions here assume the existence of a "devenv" script, which is not publicly available (something I wrote at and for work, so I currently can't publish it). However, this is only a fully optional convenience wrapper around docker / compose. Where `devenv` is used in the instructions here you can instead do something like (get the actual current container names with `docker ls`) 
```bash
docker compose up -d
docker exec --interactive --tty mdagile-dev-container-1 "bash"
```
**Alternatively, you can also just do everything on the host** - likely the most convenient option if you are already set up for development with rust. 

Note: `build-container` runs via `docker compose run --rm`, i.e. a fresh container filesystem every invocation. Cargo's crates.io registry index/cache is persisted across these runs in a named Docker volume (`cargo-registry-cache`, mounted at `$CARGO_HOME/registry`), so only the very first build after cloning (or after `docker volume rm`/`docker compose down -v`) needs to fetch the index and download crates from the network.

### Essential Commands


(run in container or host as preferred)
```bash
cargo build                      # Build
cargo run --bin agile            # Run CLI
./target/debug/agilels           # Run LSP (runs on stdin/stdout) - not really useful in this form, unless you are an AI or speak LSP :)

cargo test                       # Run full test suite
cargo test --lib -- test_name    # Run single test
```

#### Checking for warnings

The `mdagile-gui` crate has a non-default `server` feature (`dioxus/server`, `mdagile`, `dioxus-cli-config`) gating `crates/gui/src/server/mod.rs`, which is the sole caller of several `crates/gui/src/settings.rs`/`crates/gui/src/lock.rs` functions. Running plain `cargo check`/`cargo check --workspace` (default features only) therefore reports those functions as dead code — this is a build-config artifact, not a real issue. To get an accurate warning picture for the GUI crate, always check with the `server` feature enabled:
```bash
cargo check -p mdagile-gui --features server
```
Only treat warnings as real/actionable if they still appear with `--features server` included.

### GUI

run board (GUI):
```
(cd crates/gui && MDAGILE_WORKDIR=$(pwd)/../.. dx serve --platform web)
```
Then connect to the served page (usually `http://127.0.0.1:8080/`) in your browser. 

Test scripts, to simulate card movement, using a temporary directory with fake tasks:

```sh
./scripts/gui-board-demo.sh 
# or
./scripts/gui-board-demo.sh many
```

Hacky install for gui:
```
devenv . -a -c "cd crates/gui && dx bundle" && cp target/dx/mdagile-gui/debug/web/server mdagile-gui && cp -r target/dx/mdagile-gui/debug/web/public/ .
```
(alt: use --release flag and take binary from dir "..release..") 

The server is then callable like so (on the host)
```
MDAGILE_WORKDIR=.. ./mdagile-gui
```
Hacky install for cli + ls:
```
devenv . --no-tty -a -c "cargo install --path crates/cli" && cp target/release/agilels ~/.local/bin && cp target/release/agile ~/.local/bin/
```

# Full (hacky) installation:

What it does:
- build project
- copy bins to project root
- create convenient symlinks to the executables (adding the commands to the users path)
- chmod +x
```
devenv . -a -c "cd crates/gui && dx bundle" && cp target/dx/mdagile-gui/debug/web/server agilegui && cp -r target/dx/mdagile-gui/debug/web/public/ . && cargo install --path crates/cli && cp target/release/agilels . && cp target/release/agile . ; ln -sf $(pwd)/agilels ~/.local/bin/agilels ; ln -sf $(pwd)/agile ~/.local/bin/agile ; ln -sf $(pwd)/agilegui ~/.local/bin/agilegui ; chmod +x ~/.local/bin/agilels ; chmod +x ~/.local/bin/agile ; chmod +x ~/.local/bin/agilegui
```

