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
fn mdagile_toml_takes_precedence_over_dotfile() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mdagile.toml"), "[Properties.feature]\n").unwrap();
    std::fs::write(dir.path().join(".mdagile.toml"), "[Properties.bug]\n").unwrap();
    let config = Config::load(dir.path()).unwrap();
    assert!(config.properties.contains_key("feature"));
    assert!(!config.properties.contains_key("bug"));
}
