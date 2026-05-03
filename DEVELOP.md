## Building & Testing

```bash
cargo build                      # Build CLI and LSP
cargo run                        # Run CLI
./target/debug/agilels          # Run LSP (runs on stdin/stdout)

cargo test                       # Run full test suite
cargo test --lib -- test_name   # Run single test
```

run board (GUI):
```
(cd crates/gui && MDAGILE_WORKDIR=$(pwd)/../.. dx serve --platform web)
```
Then connect to the served page (usually `http://127.0.0.1:8080/`) in your browser. 
