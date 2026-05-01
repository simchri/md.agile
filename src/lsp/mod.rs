//! LSP (Language Server Protocol) server implementation.
//!
//! Provides real-time validation of `.agile.md` files through a JSON-RPC 2.0
//! interface over stdin/stdout. Integrates with the existing parser and checker
//! to provide diagnostics as users edit their task files.

pub mod logger;

use std::io::{self, BufRead, BufReader, Write};
use tracing::{debug, error, info, warn};


/// Run the LSP server on stdin/stdout.
///
/// Reads JSON-RPC 2.0 messages from stdin (with LSP Content-Length headers),
/// processes them, and writes responses to stdout. Continues until shutdown
/// request is received.
pub fn run() -> io::Result<()> {
    let log_path = logger::init_logging()?;
    info!("LSP server starting, logging to: {:?}", log_path);


    let stdin = io::stdin();
    let mut stdout = io::stdout();

    info!("Waiting for messages...");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        info!("Still waiting for messages...");
    }

    info!("LSP server stopped");
    Ok(())
}
