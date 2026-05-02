//! Orchestrator for `agile check`.
//!
//! Runs every rule defined in [`crate::rules`] against the parsed
//! `&[FileItem]` and concatenates the results into a single `Vec<Issue>`.
//! New rules are added by appending to [`run`].

use crate::parser::FileItem;
use crate::rules::{self, Issue};

/// Runs all checker rules over `items` and returns the collected issues.
///
/// Issues are returned in the order their producing rule emits them. An empty
/// result means the input is clean.
pub fn run(items: &[FileItem]) -> Vec<Issue> {
    rules::check_all(items)
}

#[cfg(test)]
mod tests;
