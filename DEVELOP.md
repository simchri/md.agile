## Building & Testing

```bash
cargo build                      # Build CLI and LSP
cargo run                        # Run CLI
./target/debug/agilels          # Run LSP (runs on stdin/stdout)

cargo test                       # Run full test suite
cargo test --lib -- test_name   # Run single test
```

run board (GUI): `devenv .`, then
```
(cd crates/gui && MDAGILE_WORKDIR=$(pwd)/../.. dx serve --platform web)
```
Then connect to the served page (usually `http://127.0.0.1:8080/`) in your browser. 

Test scripts, to simulate card movement, using a temporary directory with fake tasks:

```sh
./scripts/demo.sh 
# or
./scripts/demo.sh many
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
