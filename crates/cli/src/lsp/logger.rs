//! File-based logging for the LSP server.
//!
//! Writes log records to a file in the system temp directory, since
//! stdout/stderr are reserved for the LSP protocol itself.

use env_logger::{Builder, Env, Target};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;

/// Initialize file-based logging for the LSP server.
///
/// Creates (or appends to) `agilels.log` in the system temp directory and
/// returns its path.
///
/// # Errors
/// Returns an error if the log file cannot be opened.
pub fn init_logging() -> io::Result<PathBuf> {
    let log_path = std::env::temp_dir().join("agilels.log");
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let env = Env::default().filter_or("AGILELS_LOG", "info");
    let _ = Builder::from_env(env)
        .target(Target::Pipe(Box::new(log_file)))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {} - {}",
                buf.timestamp(),
                record.level(),
                record.target(),
                record.args()
            )
        })
        .try_init();

    Ok(log_path)
}

#[cfg(test)]
mod tests;
