//! Orchestrator for `agile check`.
//!
//! Runs every rule defined in [`crate::rules`] against the parsed
//! `&[FileItem]` and concatenates the results into a single `Vec<Issue>`.
//! New rules are added by appending to [`run`].

use crate::cli::common::find_task_files;
use crate::config::Config;
use crate::parser::{self, FileItem};
use crate::rules::{self, Issue, ResolvedIdentity};
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
/// Unlike [`run`], this needs git: it compares each file's base version
/// (`base_ref`, defaulting to `HEAD` when `None`) against its current
/// working-copy version to detect tasks that just transitioned to `[x]`, and
/// resolves the acting identity — either `identity_override` (a literal
/// `[Users.X]` key, e.g. from a CI `--as` flag) or the current user's `git
/// config` identity. The check is silently skipped (returns an empty `Vec`)
/// only when there's no `identity_override` and either `root` isn't inside a
/// git repo, or no git identity can be determined at all (`git config
/// user.email`/`user.name` both empty) — see the "assignment / completion
/// validation" plan in tasks.agile.md for the rationale. If an identity *is*
/// determined (via override or git) but doesn't match any `[Users.X]` entry,
/// the check still runs and treats it as always unauthorized for any assigned
/// task.
///
/// Returns `Err` if `base_ref` is `Some` but doesn't resolve to a valid git
/// ref/commit — this is a hard usage error (distinct from authorization
/// issues), since a typo'd `--base` should never be silently ignored.
pub fn check_authorization(
    root: &Path,
    config: &Config,
    identity_override: Option<&str>,
    base_ref: Option<&str>,
) -> Result<Vec<Issue>, crate::git::InvalidRef> {
    if let Some(base) = base_ref {
        if !crate::git::ref_exists(root, base) {
            return Err(crate::git::InvalidRef(base.to_string()));
        }
    }
    let base_ref = base_ref.unwrap_or("HEAD");

    let Some(identity) = resolve_repo_identity(root, config, identity_override) else {
        return Ok(vec![]);
    };

    let mut issues = Vec::new();
    for path in find_task_files(root) {
        let Ok(new_content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let new_items = parser::parse(&new_content, path.clone());
        issues.extend(unauthorized_completion_for_file(
            root, &path, &new_items, config, &identity, base_ref,
        ));
    }
    Ok(issues)
}

/// Like [`check_authorization`], but validates a single document's in-editor
/// buffer `text` (which may be unsaved / differ from what's on disk) against
/// its `HEAD` version. Used by the LSP server, which validates on every
/// `didOpen`/`didChange` without necessarily having saved the file. Always
/// uses the live git identity and `HEAD` — there's no `--as`/`--base`
/// override support in the editor-integration path.
pub fn check_authorization_for_document(
    root: &Path,
    path: &Path,
    text: &str,
    config: &Config,
) -> Vec<Issue> {
    let Some(identity) = resolve_repo_identity(root, config, None) else {
        return vec![];
    };
    let new_items = parser::parse(text, path.to_path_buf());
    unauthorized_completion_for_file(root, path, &new_items, config, &identity, "HEAD")
}

/// Resolves the acting identity for the E013 check.
///
/// If `identity_override` is `Some`, it's looked up as a literal `[Users.X]`
/// config key: a hit yields `ResolvedIdentity::Known`, a miss yields
/// `ResolvedIdentity::Unrecognized` (no email/`git_names` fallback matching —
/// unlike the git-derived path below, an override is expected to name a
/// config key directly). Otherwise, falls back to the live git identity (as
/// seen from `root`): returns `None` (full skip) only if `root` isn't a git
/// repo, or no git identity can be determined at all. If a git identity is
/// determined but doesn't match any `[Users.X]` entry, returns
/// `Some(ResolvedIdentity::Unrecognized)` rather than skipping — the caller
/// must still run the check in that case.
fn resolve_repo_identity(
    root: &Path,
    config: &Config,
    identity_override: Option<&str>,
) -> Option<ResolvedIdentity> {
    if let Some(key) = identity_override {
        return Some(if config.users.contains_key(key) {
            ResolvedIdentity::Known(key.to_string())
        } else {
            ResolvedIdentity::Unrecognized
        });
    }

    if !crate::git::is_git_repo(root) {
        return None;
    }
    let identity = crate::git::current_identity(root)?;
    Some(
        crate::git::resolve_identity_user(config, &identity)
            .map(ResolvedIdentity::Known)
            .unwrap_or(ResolvedIdentity::Unrecognized),
    )
}

/// Fetches `path`'s version at `base_ref` (relative to `root`) and runs the
/// E013 rule comparing it against `new_items`.
fn unauthorized_completion_for_file(
    root: &Path,
    path: &Path,
    new_items: &[FileItem],
    config: &Config,
    identity: &ResolvedIdentity,
    base_ref: &str,
) -> Vec<Issue> {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let old_items = crate::git::file_content_at_ref(root, base_ref, relative)
        .map(|content| parser::parse(&content, path.to_path_buf()));

    rules::unauthorized_completion(old_items.as_deref(), new_items, config, identity)
}

#[cfg(test)]
mod tests;
