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
