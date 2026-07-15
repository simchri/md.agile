//! `agile when` — ETA and velocity-related reporting.

use crate::config::Config;
use crate::eta;
use std::path::Path;

/// `agile when` entry point.
///
/// For now, only `--velocity` is implemented.
pub fn run(root: &Path, _config: &Config, next: Option<usize>, velocity: bool) {
    if next.is_some() {
        log::error!("`agile when --next <rank>` is not implemented yet");
        std::process::exit(1);
    }

    if !velocity {
        log::error!("`agile when` is not implemented yet; use `agile when --velocity`");
        std::process::exit(1);
    }

    match eta::estimate_velocity(root) {
        Some(value) => println!("{value:.2} weight/day"),
        None => println!("unknown"),
    }
}
