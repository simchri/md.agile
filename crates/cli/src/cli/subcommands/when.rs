//! `agile when` — ETA and velocity-related reporting.

use crate::config::Config;
use crate::eta;
use std::path::Path;

/// `agile when` entry point.
///
/// Supports `--velocity [--next <rank>] [--last <days>]` (defaults to
/// `--next 1`, i.e. the next milestone) and terminal plotting via `--plot
/// [--next <rank>]` (same default), plus `--data` to show the same
/// underlying data as a raw table of task counts. With no flags, lists the
/// ETA time span for every future milestone, in backlog order.
pub fn run(
    root: &Path,
    _config: &Config,
    next: Option<usize>,
    velocity: bool,
    plot: bool,
    data: bool,
    fit: bool,
    ascii: bool,
    last_days: Option<u32>,
) {
    if plot || data {
        let rank = next.unwrap_or(1);
        let plot = match eta::build_todo_done_plot(root, rank) {
            Ok(plot) => plot,
            Err(msg) => {
                log::error!("{msg}");
                std::process::exit(1);
            }
        };
        if data {
            print!("{}", eta::render_todo_done_data(&plot));
        } else {
            print!("{}", eta::render_todo_done_plot(&plot, fit, ascii));
        }
        return;
    }

    if velocity {
        let rank = next.unwrap_or(1);
        match eta::estimate_velocity_with_window(root, rank, last_days) {
            Ok(estimate) => print!("{}", eta::render_velocity_text(estimate)),
            Err(msg) => {
                log::error!("{msg}");
                std::process::exit(1);
            }
        }
        return;
    }

    if next.is_some() {
        log::error!(
            "`agile when --next <rank>` (detail mode, without --plot/--data/--velocity) is not implemented yet"
        );
        std::process::exit(1);
    }

    match eta::build_when_report(root) {
        Ok(report) => print!("{report}"),
        Err(msg) => {
            log::error!("{msg}");
            std::process::exit(1);
        }
    }
}
