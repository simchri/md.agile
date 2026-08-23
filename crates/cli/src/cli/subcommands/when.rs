//! `agile when` — ETA and velocity-related reporting.

use crate::config::Config;
use crate::eta::{self, TrendFitAlgorithm};
use std::path::Path;

/// `agile when` entry point.
///
/// Supports `--velocity [--next <rank>] [--last <days>]` (defaults to
/// `--next 1`, i.e. the next milestone) and terminal plotting via `--plot
/// [--next <rank>] [--last <days>]` (same default), plus `--data` to show
/// the same underlying data as a raw table of task counts, and `--html`
/// to write a self-contained HTML/SVG chart file instead of printing to
/// the terminal. `--last` restricts the historical data used for the
/// ETA/velocity calculation the same way in every mode, including the
/// bare list and `--next <rank>` detail modes (no `--velocity`/`--plot`/
/// `--data` required). With no flags, lists the ETA time span for every
/// future milestone, in backlog order.
#[allow(clippy::too_many_arguments)]
pub fn run(
    root: &Path,
    _config: &Config,
    next: Option<usize>,
    velocity: bool,
    plot: bool,
    data: bool,
    extra: f64,
    ascii: bool,
    html: bool,
    no_color: bool,
    last_days: Option<u32>,
    algorithm: TrendFitAlgorithm,
) {
    if plot || data {
        let rank = next.unwrap_or(1);
        let mut plot = match eta::build_todo_done_plot(root, rank) {
            Ok(plot) => plot,
            Err(msg) => {
                log::error!("{msg}");
                std::process::exit(1);
            }
        };
        eta::restrict_to_window_days(&mut plot, last_days);
        if data {
            print!("{}", eta::render_todo_done_data(&plot));
        } else if html {
            match eta::write_todo_done_plot_html(root, &plot, extra, algorithm) {
                Ok(path) => println!("Wrote {}", path.display()),
                Err(msg) => {
                    log::error!("{msg}");
                    std::process::exit(1);
                }
            }
        } else {
            print!(
                "{}",
                eta::render_todo_done_plot(&plot, extra, ascii, !no_color, algorithm)
            );
        }
        return;
    }

    if velocity {
        let rank = next.unwrap_or(1);
        match eta::estimate_velocity_with_window(root, rank, last_days, algorithm) {
            Ok(estimate) => print!("{}", eta::render_velocity_text(estimate)),
            Err(msg) => {
                log::error!("{msg}");
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(rank) = next {
        match eta::build_when_detail_report(root, rank, algorithm, last_days) {
            Ok(report) => print!("{report}"),
            Err(msg) => {
                log::error!("{msg}");
                std::process::exit(1);
            }
        }
        return;
    }

    match eta::build_when_report(root, algorithm, last_days) {
        Ok(report) => print!("{report}"),
        Err(msg) => {
            log::error!("{msg}");
            std::process::exit(1);
        }
    }
}
