use crate::config::{CONFIG_FILE_NAMES, ConfigError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tower_lsp::lsp_types::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Source {
    pub path: PathBuf,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Snapshot {
    Absent,
    Present(Source),
    Error(String),
}

pub(super) fn snapshot(source: &Result<Option<Source>, ConfigError>) -> Snapshot {
    match source {
        Ok(Some(source)) => Snapshot::Present(source.clone()),
        Ok(None) => Snapshot::Absent,
        Err(error) => Snapshot::Error(error.to_string()),
    }
}

/// Resolve the editor-visible config. A root restricts discovery to the
/// project directory; without one, the nearest ancestor config wins.
pub(super) fn resolve(
    root: Option<&Path>,
    task_path: &Path,
    docs: &HashMap<Url, String>,
) -> Result<Option<Source>, ConfigError> {
    if let Some(root) = root {
        return in_dir(root, docs);
    }
    for dir in task_path.parent().into_iter().flat_map(Path::ancestors) {
        if let Some(source) = in_dir(dir, docs)? {
            return Ok(Some(source));
        }
    }
    Ok(None)
}

fn in_dir(dir: &Path, docs: &HashMap<Url, String>) -> Result<Option<Source>, ConfigError> {
    let paths = CONFIG_FILE_NAMES.map(|name| dir.join(name));
    let present = paths.each_ref().map(|path| {
        path.exists()
            || Url::from_file_path(&path)
                .ok()
                .is_some_and(|uri| docs.contains_key(&uri))
    });
    if present.iter().all(|exists| *exists) {
        return Err(ConfigError::ConflictingConfig { paths });
    }
    let Some(path) = paths
        .into_iter()
        .zip(present)
        .find_map(|(path, present)| present.then_some(path))
    else {
        return Ok(None);
    };
    let text = Url::from_file_path(&path)
        .ok()
        .and_then(|uri| docs.get(&uri).cloned())
        .map(Ok)
        .unwrap_or_else(|| std::fs::read_to_string(&path))?;
    Ok(Some(Source { path, text }))
}

#[cfg(test)]
#[path = "config_source_tests.rs"]
mod tests;
