//! Logging initialization for the `agile` CLI binary.
//!
//! Routes user-facing diagnostic output (errors, warnings, informational
//! messages) through the `log` facade so it is leveled and filterable. Output
//! goes to stderr so commands whose stdout is meant to be machine-readable
//! (e.g. `agile list`, `agile task next`) stay clean.
//!
//! Verbosity is controlled via the `AGILE_LOG` environment variable using the
//! same syntax as `RUST_LOG`. Defaults to `info`.

use env_logger::{Builder, Env, Target};
use std::io::Write;

/// Installs a stderr-writing `env_logger` for the CLI.
///
/// Uses `try_init`, so duplicate calls (e.g. from tests) are silently ignored
/// rather than panicking.
pub fn init() {
    let env = Env::default().filter_or("AGILE_LOG", "info");
    let _ = Builder::from_env(env)
        .target(Target::Stderr)
        .format(|buf, record| writeln!(buf, "{} {}", record.level(), record.args()))
        .try_init();
}
