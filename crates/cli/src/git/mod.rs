//! Thin wrapper around the `git` CLI binary.
//!
//! Used by the "assignment / completion validation" check to resolve the
//! current user's git identity and to fetch the last-committed (`HEAD`)
//! version of a file for diffing against the working copy. Shells out to
//! `git` rather than adding a library dependency (`git2`), since `git` is
//! already a required tool in this project's environment.

use std::path::Path;
use std::process::Command;

/// The current user's git identity, as reported by `git config`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GitIdentity {
    pub email: Option<String>,
    pub name: Option<String>,
}

/// Returns `true` if `dir` is inside a git working tree.
pub fn is_git_repo(dir: &Path) -> bool {
    run_git(dir, &["rev-parse", "--is-inside-work-tree"])
        .map(|out| out.trim() == "true")
        .unwrap_or(false)
}

/// Resolves the current git identity (`user.email` / `user.name`) as seen from
/// `dir`. Returns `None` if neither value is configured.
pub fn current_identity(dir: &Path) -> Option<GitIdentity> {
    let email = run_git(dir, &["config", "user.email"]).map(|s| s.trim().to_string());
    let name = run_git(dir, &["config", "user.name"]).map(|s| s.trim().to_string());

    let email = email.filter(|s| !s.is_empty());
    let name = name.filter(|s| !s.is_empty());

    if email.is_none() && name.is_none() {
        None
    } else {
        Some(GitIdentity { email, name })
    }
}

/// Fetches the `HEAD` (last committed) content of `relative_path` (relative to
/// the repo, typically also relative to `dir`). Returns `None` if the file has
/// no committed version (untracked, new file, or the repo has no commits yet).
pub fn head_file_content(dir: &Path, relative_path: &Path) -> Option<String> {
    let spec = format!("HEAD:{}", relative_path.to_string_lossy());
    run_git(dir, &["show", &spec])
}

/// Runs `git` with `args` in `dir`, returning stdout as a `String` on success
/// (exit code 0), or `None` on any failure (non-zero exit, missing binary,
/// invalid UTF-8).
fn run_git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Resolves a [`GitIdentity`] to the `[Users.X]` config key that identifies it.
///
/// Tries an email match first (against every user's `git_emails`); if none match,
/// falls back to a `user.name` match against `git_names`. Returns `None` if
/// neither the identity nor the config yields a match.
pub fn resolve_identity_user(
    config: &crate::config::Config,
    identity: &GitIdentity,
) -> Option<String> {
    if let Some(email) = &identity.email {
        for (key, user) in &config.users {
            if user.git_emails.iter().any(|e| e == email) {
                return Some(key.clone());
            }
        }
    }
    if let Some(name) = &identity.name {
        for (key, user) in &config.users {
            if user.git_names.iter().any(|n| n == name) {
                return Some(key.clone());
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
