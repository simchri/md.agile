use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub properties: HashMap<String, PropertyConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyConfig {
    pub name: String,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "could not read config file: {e}"),
            ConfigError::Parse(e) => write!(f, "invalid config: {e}"),
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
}

impl Config {
    pub fn from_str(s: &str) -> Result<Self, toml::de::Error> {
        let raw: RawConfig = toml::from_str(s)?;
        let properties = raw
            .properties
            .into_keys()
            .map(|name| (name.clone(), PropertyConfig { name }))
            .collect();
        Ok(Config { properties })
    }

    pub fn load(root: &Path) -> Result<Self, ConfigError> {
        for name in &["mdagile.toml", ".mdagile.toml"] {
            let path = root.join(name);
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                return Config::from_str(&content).map_err(ConfigError::Parse);
            }
        }
        Ok(Config::default())
    }
}

#[cfg(test)]
mod tests;
