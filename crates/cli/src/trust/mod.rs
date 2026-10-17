//! Per-user trust store for form validation scripts (see README.vision.md
//! "Forms & Fields"). Running a script is a trust decision: an entry here
//! approves one script, identified by its project-relative path plus the
//! sha256 of its content, so editing the script revokes trust. The store
//! lives outside the repository so a cloned repo can't pre-trust itself, and
//! the key deliberately excludes the absolute project path, which differs
//! per CI runner.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct TrustEntry {
    script: String,
    sha256: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TrustStore {
    trusted: BTreeSet<TrustEntry>,
}

#[derive(Debug)]
pub enum TrustError {
    Io(std::io::Error),
    Malformed(serde_json::Error),
}

impl std::fmt::Display for TrustError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustError::Io(e) => write!(f, "could not access trust store: {e}"),
            TrustError::Malformed(e) => write!(f, "trust store is malformed: {e}"),
        }
    }
}

impl TrustStore {
    /// A missing file is an empty store; an unreadable or malformed one is an error.
    pub fn load(path: &Path) -> Result<Self, TrustError> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).map_err(TrustError::Malformed),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(TrustError::Io(e)),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), TrustError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(TrustError::Io)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(TrustError::Malformed)?;
        std::fs::write(path, json + "\n").map_err(TrustError::Io)
    }

    pub fn is_trusted(&self, script: &str, sha256: &str) -> bool {
        self.trusted.contains(&TrustEntry {
            script: script.to_string(),
            sha256: sha256.to_string(),
        })
    }

    pub fn trust(&mut self, script: &str, sha256: &str) {
        self.trusted.insert(TrustEntry {
            script: script.to_string(),
            sha256: sha256.to_string(),
        });
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `$XDG_CONFIG_HOME/mdagile/trust.json`, else `$HOME/.config/mdagile/trust.json`.
pub fn default_store_path(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let config_home = env("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env("HOME")
                .filter(|v| !v.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        })?;
    Some(config_home.join("mdagile").join("trust.json"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptPathError {
    Absolute,
    OutsideProject,
    NotFound,
}

impl std::fmt::Display for ScriptPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ScriptPathError::Absolute => "absolute paths are not allowed",
            ScriptPathError::OutsideProject => "the path must stay inside the project",
            ScriptPathError::NotFound => "no such file",
        })
    }
}

/// Resolves a project-relative script path to the file it names, refusing
/// absolute paths, `..` traversal and symlinks that escape the project.
pub fn resolve_script(root: &Path, script: &str) -> Result<PathBuf, ScriptPathError> {
    let relative = Path::new(script);
    if relative.is_absolute() {
        return Err(ScriptPathError::Absolute);
    }
    if relative
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(ScriptPathError::OutsideProject);
    }
    let resolved = root
        .join(relative)
        .canonicalize()
        .map_err(|_| ScriptPathError::NotFound)?;
    let root = root.canonicalize().map_err(|_| ScriptPathError::NotFound)?;
    if !resolved.starts_with(&root) {
        return Err(ScriptPathError::OutsideProject);
    }
    if !resolved.is_file() {
        return Err(ScriptPathError::NotFound);
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests;
