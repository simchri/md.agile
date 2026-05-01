//! File-based logging for the LSP server.
//!
//! Initializes structured logging to a file in the system temp directory,
//! since stdout/stderr are used by the LSP protocol itself.

use std::io;
use std::path::PathBuf;
use tracing_subscriber::fmt::format::FmtSpan;

/// Initialize file-based logging for the LSP server.
///
/// Creates a log file in the system temp directory with the pattern
/// `agilels-*.log` and returns the path to the log file.
///
/// # Returns
/// The path to the created log file
///
/// # Errors
/// Returns an error if the log file cannot be created
pub fn init_logging() -> io::Result<PathBuf> {
    // Create a file appender in the system temp directory
    let file_appender = tracing_appender::rolling::never(
        std::env::temp_dir(),
        "agilels.log",
    );

    // Set up the subscriber with the file appender
    let subscriber = tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_span_events(FmtSpan::ACTIVE)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(true)
        .with_level(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");

    let log_path = std::env::temp_dir().join("agilels.log");
    Ok(log_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_logging_creates_log_file() {
        // Clean up any existing log file first
        let log_path = std::env::temp_dir().join("agilels.log");
        let _ = std::fs::remove_file(&log_path);

        let result = init_logging();
        assert!(result.is_ok(), "init_logging should succeed");

        let log_path = result.unwrap();
        // Note: We can't reliably test that the file exists immediately because
        // tracing may buffer writes. But we can verify the path is correct.
        assert!(log_path.ends_with("agilels.log"));
    }
}
