//! `agile check` — validates task files against the built-in rule set.

use crate::checker;
use crate::cli::common::{find_task_files, parse_files};
use crate::formatter;
use std::path::Path;

/// `agile check` entry point. Prints issues to stdout and exits 1 if any are found.
pub fn run(root: &Path) {
    let items = parse_files(&find_task_files(root));
    let issues = checker::run(&items);
    for issue in &issues {
        print!("{}", formatter::format_issue(issue));
    }
    if !issues.is_empty() {
        std::process::exit(1);
    }
}
