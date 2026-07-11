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
    let prop = config.properties.get("feature").unwrap();
    assert_eq!(prop.subtasks, vec!["dev implementation", "test"]);
}

#[test]
fn property_without_subtasks_has_empty_vec() {
    let config = Config::from_str("[Properties.bug]\n").unwrap();
    let prop = config.properties.get("bug").unwrap();
    assert!(prop.subtasks.is_empty());
}

#[test]
fn property_without_subtasks_allow_cancel_has_empty_vec() {
    let config = Config::from_str("[Properties.bug]\n").unwrap();
    let prop = config.properties.get("bug").unwrap();
    assert!(prop.subtasks_allow_cancel.is_empty());
}

#[test]
fn property_with_subtasks_allow_cancel_field_is_parsed() {
    let input = "\
[Properties.feature]
subtasks = [\"dev implementation\", \"test\"]
subtasks_allow_cancel = [false, true]
";
    let config = Config::from_str(input).unwrap();
    let prop = config.properties.get("feature").unwrap();
    assert_eq!(prop.subtasks_allow_cancel, vec![false, true]);
}

#[test]
fn mismatched_subtasks_allow_cancel_length_is_an_error() {
    let input = "\
[Properties.feature]
subtasks = [\"dev implementation\", \"test\"]
subtasks_allow_cancel = [false]
";
    let err = Config::from_str(input).unwrap_err();
    assert!(matches!(err, ConfigError::PropertyValidation { .. }));
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

// ── user identity (emails / git_names) ─────────────────────────────────────────

#[test]
fn user_without_emails_or_git_names_has_empty_vecs() {
    let config = Config::from_str("[Users.alice]\n").unwrap();
    let user = config.users.get("alice").unwrap();
    assert!(user.emails.is_empty());
    assert!(user.git_names.is_empty());
}

#[test]
fn user_with_emails_is_parsed() {
    let input = "\
[Users.alice]
emails = [\"alice@example.com\", \"a@example.org\"]
";
    let config = Config::from_str(input).unwrap();
    let user = config.users.get("alice").unwrap();
    assert_eq!(
        user.emails,
        vec!["alice@example.com".to_string(), "a@example.org".to_string()]
    );
}

#[test]
fn user_with_git_names_is_parsed() {
    let input = "\
[Users.alice]
git_names = [\"Alice Smith\"]
";
    let config = Config::from_str(input).unwrap();
    let user = config.users.get("alice").unwrap();
    assert_eq!(user.git_names, vec!["Alice Smith".to_string()]);
}

// ── group membership ───────────────────────────────────────────────────────────

#[test]
fn group_without_members_has_empty_vec() {
    let config = Config::from_str("[Groups.devs]\n").unwrap();
    let group = config.groups.get("devs").unwrap();
    assert!(group.members.is_empty());
}

#[test]
fn group_with_members_is_parsed() {
    let input = "\
[Groups.devs]
members = [\"alice\", \"bob\"]
";
    let config = Config::from_str(input).unwrap();
    let group = config.groups.get("devs").unwrap();
    assert_eq!(group.members, vec!["alice".to_string(), "bob".to_string()]);
}

// ── unknown key validation ─────────────────────────────────────────────────────

#[test]
fn unknown_top_level_section_is_rejected() {
    let input = "\
[Typo]
foo = 1
";
    let result = Config::from_str(input);
    assert!(matches!(result, Err(ConfigError::Parse(_))));
}

#[test]
fn unknown_key_in_property_is_rejected() {
    let input = "\
[Properties.feature]
subtsaks = [\"dev implementation\"]
";
    let result = Config::from_str(input);
    assert!(matches!(result, Err(ConfigError::Parse(_))));
}

#[test]
fn unknown_key_in_user_is_rejected() {
    let input = "\
[Users.alice]
emial = \"alice@example.com\"
";
    let result = Config::from_str(input);
    assert!(matches!(result, Err(ConfigError::Parse(_))));
}

#[test]
fn unknown_key_in_group_is_rejected() {
    let input = "\
[Groups.devs]
memebrs = [\"alice\"]
";
    let result = Config::from_str(input);
    assert!(matches!(result, Err(ConfigError::Parse(_))));
}

#[test]
fn known_keys_are_still_accepted() {
    let input = "\
[Properties.feature]
subtasks = [\"dev implementation\"]
subtasks_allow_cancel = [true]

[Users.alice]
emails = [\"alice@example.com\"]
git_names = [\"Alice\"]

[Groups.devs]
members = [\"alice\"]
";
    assert!(Config::from_str(input).is_ok());
}
