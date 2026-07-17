//! `agile history` — list currently closed tasks with completion dates.

use crate::cli::common::{find_task_files, parse_file};
use crate::eta::{self, StatusTransition};
use crate::history_cache;
use crate::lifecycle_cache;
use crate::parser::Status;
use std::path::Path;

/// `agile history` entry point.
pub fn run(root: &Path) {
    let _ = history_cache::update(root);
    let mut out = String::new();
    for file in find_task_files(root) {
        let items = parse_file(&file);
        let current_nodes = eta::status_transitions(&[], &items);
        if current_nodes.is_empty() {
            continue;
        }

        let completion_dates =
            lifecycle_cache::completion_dates_for_current_file(root, &file, &items);
        for node in current_nodes
            .iter()
            .filter(|n| matches!(n.new_status, Status::Done | Status::Cancelled))
        {
            let date = completion_dates
                .get(&node.key)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            out.push_str(&render_history_line(&date, node));
        }
    }
    print!("{out}");
}

fn render_history_line(date: &str, node: &StatusTransition) -> String {
    format!(
        "{date} {}- {} {}\n",
        " ".repeat(node.indent),
        status_marker(&node.new_status),
        node.title
    )
}

fn status_marker(status: &Status) -> &'static str {
    match status {
        Status::Todo => "[ ]",
        Status::Done => "[x]",
        Status::Cancelled => "[-]",
    }
}

#[cfg(test)]
fn unix_to_yyyy_mm_dd(unix_ts: i64) -> String {
    let days = unix_ts.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

// Converts days since Unix epoch to a Gregorian date in UTC.
#[cfg(test)]
fn civil_from_days(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = mp + if mp < 10 { 3 } else { -9 }; // [1, 12]
    if m <= 2 {
        y += 1;
    }
    (y, m, d)
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
