//! Persisted "active project" and "recent projects" settings for runtime
//! project switching (see `server::switch_project` / the "≡ menu → Switch
//! project" UI in `main.rs`).
//!
//! Stored as a small TOML file at `~/.config/mdagile-gui/settings.toml`
//! (respecting `$XDG_CONFIG_HOME` if set), read once at server startup to
//! seed the initial working directory (replacing the old
//! `MDAGILE_WORKDIR`-is-fixed-forever behavior), and rewritten on every
//! successful project switch.
//!
//! Kept target-independent (no `feature = "server"` gate), same reasoning as
//! `lock.rs`: pure, testable logic exercised by a plain `cargo test`, with
//! only its *callers* in `server/mod.rs` gated to the native server build.

use std::path::{Path, PathBuf};

/// Max number of recent project paths to remember, most-recent first.
const MAX_RECENT: usize = 10;

/// The persisted settings: the currently active project, and a most-recent-
/// first list of previously used ones (including the current one, so a
/// user can switch back to whatever they had open last across restarts).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    pub current: Option<PathBuf>,
    pub recent: Vec<PathBuf>,
}

/// Resolves the path of the settings file. Prefers `$XDG_CONFIG_HOME`;
/// falls back to `$HOME/.config` (the same fallback XDG itself specifies).
pub fn settings_file_path() -> PathBuf {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|v| !v.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    config_home.join("mdagile-gui").join("settings.toml")
}

/// Serializes `settings` into the on-disk TOML format. Paths that aren't
/// valid UTF-8 are silently dropped — an extreme edge case on the Linux
/// targets this project packages for, not worth failing the whole write
/// over.
pub fn format_settings(settings: &Settings) -> String {
    let mut out = String::new();
    if let Some(current) = settings.current.as_ref().and_then(|p| p.to_str()) {
        out.push_str("current = ");
        out.push_str(&toml::Value::String(current.to_string()).to_string());
        out.push('\n');
    }
    let recent: Vec<toml::Value> = settings
        .recent
        .iter()
        .filter_map(|p| p.to_str())
        .map(|s| toml::Value::String(s.to_string()))
        .collect();
    out.push_str("recent = ");
    out.push_str(&toml::Value::Array(recent).to_string());
    out.push('\n');
    out
}

/// Parses the on-disk TOML format back into a [`Settings`]. Missing fields
/// or a malformed file are tolerated, resolving to an empty/default
/// `Settings` rather than an error — a corrupt or half-written settings
/// file should never prevent the GUI from starting.
pub fn parse_settings(contents: &str) -> Settings {
    let Ok(value) = contents.parse::<toml::Value>() else {
        return Settings::default();
    };
    let current = value
        .get("current")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);
    let recent = value
        .get("recent")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default();
    Settings { current, recent }
}

/// Reads and parses the settings file at `path`, if present. Returns the
/// default (empty) `Settings` if the file doesn't exist or fails to parse —
/// same "never block startup" reasoning as [`parse_settings`].
pub fn read_settings(path: &Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(contents) => parse_settings(&contents),
        Err(_) => Settings::default(),
    }
}

/// Writes `settings` to the settings file at `path`, creating its parent
/// directory (e.g. `~/.config/mdagile-gui/`) if necessary.
pub fn write_settings(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format_settings(settings))
}

/// Returns the updated [`Settings`] after recording `project` as the new
/// active project: it becomes `current`, and is moved to the front of
/// `recent` (de-duplicated — a project already in the list isn't listed
/// twice, it just moves to the front), capped at [`MAX_RECENT`] entries so
/// the list can't grow unbounded over a long-lived install.
pub fn record_project(settings: &Settings, project: &Path) -> Settings {
    let recent: Vec<PathBuf> = std::iter::once(project.to_path_buf())
        .chain(settings.recent.iter().filter(|p| *p != project).cloned())
        .take(MAX_RECENT)
        .collect();

    Settings {
        current: Some(project.to_path_buf()),
        recent,
    }
}

/// Reports whether `dir` looks like a valid mdagile project root — i.e. it
/// contains an `mdagile.toml` (file or symlink; a directory literally named
/// `mdagile.toml` doesn't count).
pub fn is_project_dir(dir: &Path) -> bool {
    match std::fs::symlink_metadata(dir.join("mdagile.toml")) {
        Ok(metadata) => metadata.is_file() || metadata.file_type().is_symlink(),
        Err(_) => false,
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
