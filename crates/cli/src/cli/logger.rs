//! Logging initialization for the `agile` CLI binary.
//!
//! Routes user-facing diagnostic output (errors, warnings, informational
//! messages) through the `tracing` ecosystem so it is structured, level-aware,
//! and filterable. Output goes to stderr so commands whose stdout is meant to
//! be machine-readable (e.g. `agile list`, `agile task next`) stay clean.
//!
//! Verbosity is controlled via the `AGILE_LOG` environment variable using the
//! same syntax as `RUST_LOG`. Defaults to `info`.

use tracing_subscriber::{EnvFilter, fmt};

/// Installs a stderr-writing tracing subscriber for the CLI.
///
/// Uses `try_init`, so duplicate calls (e.g. from tests) are silently ignored
/// rather than panicking.
pub fn init() {
    let filter = EnvFilter::try_from_env("AGILE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .with_target(false)
        .compact()
        .try_init();
}
