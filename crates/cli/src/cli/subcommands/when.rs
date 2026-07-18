//! `agile when` — ETA and velocity-related reporting.

use crate::config::Config;
use crate::eta;
use std::path::Path;

/// `agile when` entry point.
///
/// Supports `--velocity` and terminal plotting via `--plot --next <rank>`.
pub fn run(
    root: &Path,
    _config: &Config,
    next: Option<usize>,
    velocity: bool,
    plot: bool,
    last_days: Option<u32>,
) {
    if plot {
        let Some(rank) = next else {
            log::error!("`agile when --plot` requires `--next <rank>`");
            std::process::exit(1);
        };
        let plot = match eta::build_todo_done_plot(root, rank) {
            Ok(plot) => plot,
            Err(msg) => {
                log::error!("{msg}");
                std::process::exit(1);
            }
        };
        print!("{}", eta::render_todo_done_plot(&plot));
        return;
    }

    if next.is_some() {
        log::error!("`agile when --next <rank>` is not implemented yet");
        std::process::exit(1);
    }

    if !velocity {
        log::error!("`agile when` is not implemented yet; use `agile when --velocity`");
        std::process::exit(1);
    }

    let window_days = last_days.unwrap_or(eta::DEFAULT_VELOCITY_WINDOW_DAYS);
    match eta::estimate_velocity_with_window(root, window_days) {
        Some(value) => println!("{value:.2} weight/day"),
        None => println!("unknown"),
    }
}
