//! Orchestrator for `agile check`.
//!
//! Runs every rule defined in [`crate::rules`] against the parsed
//! `&[FileItem]` and concatenates the results into a single `Vec<Issue>`.
//! New rules are added by appending to [`run`].

use crate::cli::common::find_task_files;
use crate::config::Config;
use crate::parser::{self, FileItem};
use crate::rules::{self, Issue};
use std::path::Path;

/// Runs all checker rules over `items` and returns the collected issues.
///
/// Issues are returned in the order their producing rule emits them. An empty
/// result means the input is clean.
pub fn run(items: &[FileItem], config: &Config) -> Vec<Issue> {
    rules::check_all(items, config)
}

/// Runs the E013 "assignment / completion validation" check across every
/// `.agile.md` file under `root`.
///
/// Unlike [`run`], this needs git: it compares each file's `HEAD` (last
/// committed) version against its current working-copy version to detect
/// tasks that just transitioned to `[x]`, and resolves the current user's
/// identity via `git config`. The check is silently skipped (returns an empty
/// `Vec`) whenever `root` isn't inside a git repo, or the current identity
/// doesn't resolve to any `[Users.X]` entry in `config` — see the "assignment
/// / completion validation" plan in tasks.agile.md for the rationale.
pub fn check_authorization(root: &Path, config: &Config) -> Vec<Issue> {
    let Some(identity_user) = resolve_repo_identity(root, config) else {
        return vec![];
    };

    let mut issues = Vec::new();
    for path in find_task_files(root) {
        let Ok(new_content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let new_items = parser::parse(&new_content, path.clone());
        issues.extend(unauthorized_completion_for_file(
            root,
            &path,
            &new_items,
            config,
            &identity_user,
        ));
    }
    issues
}

/// Like [`check_authorization`], but validates a single document's in-editor
/// buffer `text` (which may be unsaved / differ from what's on disk) against
/// its `HEAD` version. Used by the LSP server, which validates on every
/// `didOpen`/`didChange` without necessarily having saved the file.
pub fn check_authorization_for_document(
    root: &Path,
    path: &Path,
    text: &str,
    config: &Config,
) -> Vec<Issue> {
    let Some(identity_user) = resolve_repo_identity(root, config) else {
        return vec![];
    };
    let new_items = parser::parse(text, path.to_path_buf());
    unauthorized_completion_for_file(root, path, &new_items, config, &identity_user)
}

/// Resolves the current git identity (as seen from `root`) to a `[Users.X]`
/// config key. Returns `None` if `root` isn't a git repo, or the identity
/// doesn't match any configured user — in both cases the E013 check should be
/// silently skipped.
fn resolve_repo_identity(root: &Path, config: &Config) -> Option<String> {
    if !crate::git::is_git_repo(root) {
        return None;
    }
    let identity = crate::git::current_identity(root)?;
    crate::git::resolve_identity_user(config, &identity)
}

/// Fetches `path`'s `HEAD` version (relative to `root`) and runs the E013 rule
/// comparing it against `new_items`.
fn unauthorized_completion_for_file(
    root: &Path,
    path: &Path,
    new_items: &[FileItem],
    config: &Config,
    identity_user: &str,
) -> Vec<Issue> {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let old_items = crate::git::head_file_content(root, relative)
        .map(|content| parser::parse(&content, path.to_path_buf()));

    rules::unauthorized_completion(old_items.as_deref(), new_items, config, identity_user)
}

#[cfg(test)]
mod tests;
