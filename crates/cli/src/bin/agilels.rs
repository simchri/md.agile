use clap::Parser;

#[derive(Parser)]
#[command(name = "agilels", about = "Language server for mdagile")]
struct Args {
    /// Print version and exit
    #[arg(long)]
    version: bool,
}

fn main() {
    let args = Args::parse();

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if let Err(e) = mdagile::lsp::run() {
        eprintln!("LSP server error: {}", e);
        std::process::exit(1);
    }
}
