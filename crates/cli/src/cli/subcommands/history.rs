//! `agile history` — list currently closed tasks with completion dates.

use crate::cli::common::{find_task_files, parse_file};
use crate::eta::{self, StatusTransition};
use crate::lifecycle_cache;
use crate::parser::Status;
use std::path::Path;

/// `agile history` entry point.
pub fn run(root: &Path) {
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
