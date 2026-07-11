//! `agile check` — validates task files against the built-in rule set.

use crate::checker;
use crate::cli::common::{find_task_files, parse_files};
use crate::config::Config;
use crate::formatter;
use std::path::Path;

/// `agile check` entry point. Prints issues to stdout and exits 1 if any are found.
pub fn run(root: &Path, config: &Config) {
    let items = parse_files(&find_task_files(root));
    let mut issues = checker::run(&items, config);
    issues.extend(checker::check_authorization(root, config));
    for issue in &issues {
        print!("{}", formatter::format_issue(issue));
    }
    if !issues.is_empty() {
        std::process::exit(1);
    }
}
