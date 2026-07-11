//! `agile check` — validates task files against the built-in rule set.

use crate::checker;
use crate::cli::common::{find_task_files, parse_files};
use crate::config::Config;
use crate::formatter;
use std::path::Path;

/// `agile check` entry point. Prints issues to stdout and exits 1 if any are
/// found. `as_user` and `base_ref` support CI/CD use cases where the acting
/// identity and/or diff base need overriding (e.g. checking whether the
/// author of a PR is allowed to complete a task, comparing against the PR's
/// base branch rather than the local working copy's `HEAD`).
pub fn run(root: &Path, config: &Config, as_user: Option<&str>, base_ref: Option<&str>) {
    let items = parse_files(&find_task_files(root));
    let mut issues = checker::run(&items, config);
    match checker::check_authorization(root, config, as_user, base_ref) {
        Ok(authorization_issues) => issues.extend(authorization_issues),
        Err(invalid_ref) => {
            log::error!("{invalid_ref}");
            std::process::exit(1);
        }
    }
    for issue in &issues {
        print!("{}", formatter::format_issue(issue));
    }
    if !issues.is_empty() {
        std::process::exit(1);
    }
}
