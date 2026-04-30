fn main() {
    if let Err(e) = mdagile::lsp::run() {
        eprintln!("LSP server error: {}", e);
        std::process::exit(1);
    }
}
