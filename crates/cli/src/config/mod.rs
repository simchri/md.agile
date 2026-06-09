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
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupConfig {
    pub name: String,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    ConflictingConfig { paths: [std::path::PathBuf; 2] },
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

#[derive(serde::Deserialize)]
struct RawConfig {
    #[serde(rename = "Properties", default)]
    properties: HashMap<String, toml::Value>,
    #[serde(rename = "Users", default)]
    users: HashMap<String, toml::Value>,
    #[serde(rename = "Groups", default)]
    groups: HashMap<String, toml::Value>,
}

impl Config {
    pub fn from_str(s: &str) -> Result<Self, toml::de::Error> {
        let raw: RawConfig = toml::from_str(s)?;
        let properties = raw
            .properties
            .into_keys()
            .map(|name| (name.clone(), PropertyConfig { name }))
            .collect();
        let users = raw
            .users
            .into_keys()
            .map(|name| (name.clone(), UserConfig { name }))
            .collect();
        let groups = raw
            .groups
            .into_keys()
            .map(|name| (name.clone(), GroupConfig { name }))
            .collect();
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
                Config::from_str(&content).map_err(ConfigError::Parse)
            }
            (false, true) => {
                let content = std::fs::read_to_string(&dot)?;
                Config::from_str(&content).map_err(ConfigError::Parse)
            }
            (false, false) => Ok(Config::default()),
        }
    }
}

#[cfg(test)]
mod tests;
