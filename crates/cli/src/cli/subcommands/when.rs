//! `agile when` — ETA and velocity-related reporting.

use crate::config::Config;
use crate::eta;
use crate::history_cache;
use std::path::Path;

/// `agile when` entry point.
///
/// For now, only `--velocity` is implemented.
pub fn run(
    root: &Path,
    _config: &Config,
    next: Option<usize>,
    velocity: bool,
    last_days: Option<u32>,
) {
    if next.is_some() {
        log::error!("`agile when --next <rank>` is not implemented yet");
        std::process::exit(1);
    }

    if !velocity {
        log::error!("`agile when` is not implemented yet; use `agile when --velocity`");
        std::process::exit(1);
    }

    let _ = history_cache::update(root);
    let window_days = last_days.unwrap_or(eta::DEFAULT_VELOCITY_WINDOW_DAYS);
    match eta::estimate_velocity_with_window(root, window_days) {
        Some(value) => println!("{value:.2} weight/day"),
        None => println!("unknown"),
    }
}
