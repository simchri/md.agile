use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The two file names mdagile recognizes as its config file, in the order
/// they're preferred when only one is present. Exactly one of these is
/// expected per project directory; both present at once is a
/// [`ConfigError::ConflictingConfig`].
pub const CONFIG_FILE_NAMES: [&str; 2] = ["mdagile.toml", ".mdagile.toml"];

/// Returns the config file path in `dir`, if exactly one of
/// [`CONFIG_FILE_NAMES`] exists there. Doesn't distinguish "neither exists"
/// from "both exist" — callers that need to raise [`ConfigError::ConflictingConfig`]
/// should go through [`Config::load`] instead, which needs both candidate
/// paths anyway to build that error.
pub fn find_config_file_in(dir: &Path) -> Option<PathBuf> {
    CONFIG_FILE_NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}

/// Walks up from `start_dir` (inclusive) through its ancestors, returning the
/// config file path in the nearest directory that has one.
pub fn find_config_file_upwards(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = Some(start_dir);
    while let Some(d) = dir {
        if let Some(path) = find_config_file_in(d) {
            return Some(path);
        }
        dir = d.parent();
    }
    None
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub properties: HashMap<String, PropertyConfig>,
    pub users: HashMap<String, UserConfig>,
    pub groups: HashMap<String, GroupConfig>,
    pub general: GeneralConfig,
}

/// Project-wide settings that don't belong to any single property/user/group.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralConfig {
    /// Whether `agile check` logs a terminal warning when the E013
    /// assignment/completion check is skipped because the project isn't
    /// inside a git repo at all. Defaults to `true`; set to `false` in
    /// projects that intentionally don't use git, to avoid a warning on
    /// every run.
    pub warn_when_not_a_git_repo: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        GeneralConfig {
            warn_when_not_a_git_repo: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyConfig {
    pub name: String,
    pub subtasks: Vec<String>,
    /// Parallel array to `subtasks`: if `subtasks_allow_cancel[i]` is `true`, the
    /// required subtask at `subtasks[i]` may be satisfied by cancelling it instead
    /// of completing it. Empty when not configured (no required subtask may be
    /// cancelled).
    pub subtasks_allow_cancel: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserConfig {
    pub name: String,
    /// Email addresses that identify this user's git commits. Used to match
    /// against `git config user.email` for completion-authorization checks.
    pub git_emails: Vec<String>,
    /// Alternate git display names (`git config user.name`), used as a fallback
    /// identity match when no email match is found.
    pub git_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupConfig {
    pub name: String,
    /// `[Users.X]` keys that belong to this group.
    pub members: Vec<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    ConflictingConfig {
        paths: [std::path::PathBuf; 2],
    },
    /// A `[Properties.X]` entry has a `subtasks_allow_cancel` array whose length
    /// doesn't match `subtasks`.
    PropertyValidation {
        property: String,
        message: String,
    },
    /// A `[Groups.X]` entry's `members` list references a name that isn't
    /// defined as a `[Users.X]` entry.
    GroupValidation {
        group: String,
        message: String,
    },
    /// The same `git_emails`/`git_names` value is listed on more than one
    /// `[Users.X]` entry. Left unchecked, [`crate::git::resolve_identity_user`]
    /// would pick one of the matching users based on `HashMap` iteration
    /// order, which is randomized per process and therefore non-deterministic
    /// across runs.
    DuplicateIdentity {
        field: &'static str,
        value: String,
        users: Vec<String>,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "could not read config file: {e}"),
            ConfigError::Parse(e) => write!(f, "invalid config: {e}"),
            ConfigError::ConflictingConfig { paths } => write!(
                f,
                "conflicting config files '{}' and '{}': don't know which config to use",
                paths[0].display(),
                paths[1].display(),
            ),
            ConfigError::PropertyValidation { property, message } => {
                write!(f, "invalid config for property '{property}': {message}")
            }
            ConfigError::GroupValidation { group, message } => {
                write!(f, "invalid config for group '{group}': {message}")
            }
            ConfigError::DuplicateIdentity {
                field,
                value,
                users,
            } => write!(
                f,
                "'{value}' appears in {field} of multiple users: {} (each identity must map to exactly one user)",
                users.join(", "),
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::Parse(e)
    }
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawPropertyConfig {
    #[serde(default)]
    subtasks: Vec<String>,
    #[serde(default)]
    subtasks_allow_cancel: Vec<bool>,
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawUserConfig {
    #[serde(default)]
    git_emails: Vec<String>,
    #[serde(default)]
    git_names: Vec<String>,
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawGroupConfig {
    #[serde(default)]
    members: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGeneralConfig {
    #[serde(default = "default_true")]
    warn_when_not_a_git_repo: bool,
}

impl Default for RawGeneralConfig {
    fn default() -> Self {
        RawGeneralConfig {
            warn_when_not_a_git_repo: true,
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(rename = "Properties", default)]
    properties: HashMap<String, RawPropertyConfig>,
    #[serde(rename = "Users", default)]
    users: HashMap<String, RawUserConfig>,
    #[serde(rename = "Groups", default)]
    groups: HashMap<String, RawGroupConfig>,
    #[serde(rename = "General", default)]
    general: RawGeneralConfig,
}

/// Finds a `git_emails`/`git_names` value that's listed on more than one
/// user, using `values` to pick which field to check. Returns the offending
/// value plus the (sorted) keys of every user that lists it.
///
/// Deterministic regardless of `HashMap` iteration order: candidate
/// duplicates are collected first, then sorted by value before picking the
/// first one to report, so the same misconfiguration always produces the
/// same error message.
fn find_duplicate_identity(
    users: &HashMap<String, UserConfig>,
    values: impl Fn(&UserConfig) -> &[String],
) -> Option<(String, Vec<String>)> {
    let mut owners_by_value: HashMap<&str, std::collections::HashSet<&str>> = HashMap::new();
    for (key, user) in users {
        for v in values(user) {
            owners_by_value
                .entry(v.as_str())
                .or_default()
                .insert(key.as_str());
        }
    }

    let mut duplicates: Vec<(&str, Vec<&str>)> = owners_by_value
        .into_iter()
        .filter(|(_, owners)| owners.len() > 1)
        .map(|(value, owners)| {
            let mut owners: Vec<&str> = owners.into_iter().collect();
            owners.sort();
            (value, owners)
        })
        .collect();
    duplicates.sort_by(|a, b| a.0.cmp(b.0));

    duplicates.into_iter().next().map(|(value, owners)| {
        (
            value.to_string(),
            owners.into_iter().map(String::from).collect(),
        )
    })
}

impl Config {
    pub fn from_str(s: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(s)?;
        let properties = raw
            .properties
            .into_iter()
            .map(|(name, raw_prop)| {
                if !raw_prop.subtasks_allow_cancel.is_empty()
                    && raw_prop.subtasks_allow_cancel.len() != raw_prop.subtasks.len()
                {
                    return Err(ConfigError::PropertyValidation {
                        property: name.clone(),
                        message: format!(
                            "subtasks_allow_cancel has {} entries but subtasks has {}; they must match in length",
                            raw_prop.subtasks_allow_cancel.len(),
                            raw_prop.subtasks.len(),
                        ),
                    });
                }
                Ok((
                    name.clone(),
                    PropertyConfig {
                        name,
                        subtasks: raw_prop.subtasks,
                        subtasks_allow_cancel: raw_prop.subtasks_allow_cancel,
                    },
                ))
            })
            .collect::<Result<_, ConfigError>>()?;
        let users: HashMap<String, UserConfig> = raw
            .users
            .into_iter()
            .map(|(name, raw_user)| {
                (
                    name.clone(),
                    UserConfig {
                        name,
                        git_emails: raw_user.git_emails,
                        git_names: raw_user.git_names,
                    },
                )
            })
            .collect();

        if let Some((value, owners)) = find_duplicate_identity(&users, |u| &u.git_emails) {
            return Err(ConfigError::DuplicateIdentity {
                field: "git_emails",
                value,
                users: owners,
            });
        }
        if let Some((value, owners)) = find_duplicate_identity(&users, |u| &u.git_names) {
            return Err(ConfigError::DuplicateIdentity {
                field: "git_names",
                value,
                users: owners,
            });
        }

        let groups = raw
            .groups
            .into_iter()
            .map(|(name, raw_group)| {
                if let Some(unknown) = raw_group
                    .members
                    .iter()
                    .find(|member| !users.contains_key(member.as_str()))
                {
                    return Err(ConfigError::GroupValidation {
                        group: name.clone(),
                        message: format!(
                            "member '{unknown}' is not a defined '[Users.{unknown}]' entry"
                        ),
                    });
                }
                Ok((
                    name.clone(),
                    GroupConfig {
                        name,
                        members: raw_group.members,
                    },
                ))
            })
            .collect::<Result<_, ConfigError>>()?;
        Ok(Config {
            properties,
            users,
            groups,
            general: GeneralConfig {
                warn_when_not_a_git_repo: raw.general.warn_when_not_a_git_repo,
            },
        })
    }

    pub fn load(root: &Path) -> Result<Self, ConfigError> {
        let plain = root.join(CONFIG_FILE_NAMES[0]);
        let dot = root.join(CONFIG_FILE_NAMES[1]);
        match (plain.exists(), dot.exists()) {
            (true, true) => Err(ConfigError::ConflictingConfig {
                paths: [plain, dot],
            }),
            (true, false) => {
                let content = std::fs::read_to_string(&plain)?;
                Config::from_str(&content)
            }
            (false, true) => {
                let content = std::fs::read_to_string(&dot)?;
                Config::from_str(&content)
            }
            (false, false) => Ok(Config::default()),
        }
    }
}

#[cfg(test)]
mod tests;
