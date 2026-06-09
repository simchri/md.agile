use super::*;

#[test]
fn empty_toml_gives_empty_properties() {
    let config = Config::from_str("").unwrap();
    assert!(config.properties.is_empty());
}

#[test]
fn single_property_is_parsed() {
    let config = Config::from_str("[Properties.feature]\n").unwrap();
    assert_eq!(config.properties.len(), 1);
    assert!(config.properties.contains_key("feature"));
}

#[test]
fn multiple_properties_are_parsed() {
    let input = "\
[Properties.feature]

[Properties.bug]
";
    let config = Config::from_str(input).unwrap();
    assert_eq!(config.properties.len(), 2);
    assert!(config.properties.contains_key("feature"));
    assert!(config.properties.contains_key("bug"));
}

#[test]
fn property_with_subtasks_field_is_parsed() {
    let input = "\
[Properties.feature]
subtasks = [\"dev implementation\", \"test\"]
";
    let config = Config::from_str(input).unwrap();
    assert!(config.properties.contains_key("feature"));
}

#[test]
fn single_user_is_parsed() {
    let config = Config::from_str("[Users.alice]\n").unwrap();
    assert_eq!(config.users.len(), 1);
    assert!(config.users.contains_key("alice"));
}

#[test]
fn single_group_is_parsed() {
    let config = Config::from_str("[Groups.devs]\n").unwrap();
    assert_eq!(config.groups.len(), 1);
    assert!(config.groups.contains_key("devs"));
}

#[test]
fn users_and_groups_and_properties_parsed_together() {
    let input = "\
[Properties.feature]

[Users.alice]

[Groups.devs]
";
    let config = Config::from_str(input).unwrap();
    assert!(config.properties.contains_key("feature"));
    assert!(config.users.contains_key("alice"));
    assert!(config.groups.contains_key("devs"));
}

#[test]
fn empty_toml_gives_empty_users_and_groups() {
    let config = Config::from_str("").unwrap();
    assert!(config.users.is_empty());
    assert!(config.groups.is_empty());
}

#[test]
fn invalid_toml_returns_error() {
    let result = Config::from_str("[Properties.feature\n");
    assert!(result.is_err());
}

#[test]
fn missing_config_file_returns_default_config() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config::load(dir.path()).unwrap();
    assert!(config.properties.is_empty());
}

#[test]
fn config_file_is_loaded_from_disk() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mdagile.toml"), "[Properties.feature]\n").unwrap();
    let config = Config::load(dir.path()).unwrap();
    assert!(config.properties.contains_key("feature"));
}

#[test]
fn dotfile_variant_is_loaded() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".mdagile.toml"), "[Properties.bug]\n").unwrap();
    let config = Config::load(dir.path()).unwrap();
    assert!(config.properties.contains_key("bug"));
}

#[test]
fn both_config_files_present_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mdagile.toml"), "[Properties.feature]\n").unwrap();
    std::fs::write(dir.path().join(".mdagile.toml"), "[Properties.bug]\n").unwrap();
    let err = Config::load(dir.path()).unwrap_err();
    assert!(matches!(err, ConfigError::ConflictingConfig { .. }));
}
