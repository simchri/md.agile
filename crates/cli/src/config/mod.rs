use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub properties: HashMap<String, PropertyConfig>,
    pub users: HashMap<String, UserConfig>,
    pub groups: HashMap<String, GroupConfig>,
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

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(rename = "Properties", default)]
    properties: HashMap<String, RawPropertyConfig>,
    #[serde(rename = "Users", default)]
    users: HashMap<String, RawUserConfig>,
    #[serde(rename = "Groups", default)]
    groups: HashMap<String, RawGroupConfig>,
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
        })
    }

    pub fn load(root: &Path) -> Result<Self, ConfigError> {
        let plain = root.join("mdagile.toml");
        let dot = root.join(".mdagile.toml");
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
