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
fn property_brief_and_description_are_parsed() {
    let input = "\
[Properties.feature]
brief = \"New end-user visible functionality of the product\"
description = \"Not considered features are bugfixes, refactorings, and other internal changes.\"
";
    let config = Config::from_str(input).unwrap();
    let prop = config.properties.get("feature").unwrap();
    assert_eq!(
        prop.brief.as_deref(),
        Some("New end-user visible functionality of the product")
    );
    assert_eq!(
        prop.description.as_deref(),
        Some("Not considered features are bugfixes, refactorings, and other internal changes.")
    );
}

#[test]
fn property_without_brief_or_description_has_none() {
    let config = Config::from_str("[Properties.bug]\n").unwrap();
    let prop = config.properties.get("bug").unwrap();
    assert_eq!(prop.brief, None);
    assert_eq!(prop.description, None);
}

#[test]
fn property_without_subtasks_allow_cancel_has_empty_vec() {
    let config = Config::from_str("[Properties.bug]\n").unwrap();
    let prop = config.properties.get("bug").unwrap();
    assert!(prop.subtasks_allow_cancel.is_empty());
}

#[test]
fn property_with_neighbortasks_field_is_parsed() {
    let input = "\
[Properties.frontend-implementation]
neighbortasks = [\"do the back end implementation also!\"]
";
    let config = Config::from_str(input).unwrap();
    let prop = config.properties.get("frontend-implementation").unwrap();
    assert_eq!(
        prop.neighbortasks,
        vec!["do the back end implementation also!"]
    );
}

#[test]
fn property_without_neighbortasks_has_empty_vec() {
    let config = Config::from_str("[Properties.bug]\n").unwrap();
    let prop = config.properties.get("bug").unwrap();
    assert!(prop.neighbortasks.is_empty());
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
fn property_without_branches_has_empty_branches_map() {
    let config = Config::from_str("[Properties.bug]\n").unwrap();
    let prop = config.properties.get("bug").unwrap();
    assert!(prop.branches.is_empty());
}

#[test]
fn property_branch_sub_tables_are_parsed() {
    let input = "\
[Properties.review]
subtasks = [\"document review findings\"]
[Properties.review.passed]
neighbortasks = [\"publish feature\"]
[Properties.review.failed]
neighbortasks = [\"create follow up task for fixes\"]
";
    let config = Config::from_str(input).unwrap();
    let prop = config.properties.get("review").unwrap();
    assert_eq!(prop.subtasks, vec!["document review findings"]);
    assert_eq!(prop.branches.len(), 2);
    assert_eq!(
        prop.branches.get("passed").unwrap().neighbortasks,
        vec!["publish feature"]
    );
    assert_eq!(
        prop.branches.get("failed").unwrap().neighbortasks,
        vec!["create follow up task for fixes"]
    );
}

#[test]
fn property_branch_can_declare_its_own_subtasks_and_allow_cancel() {
    let input = "\
[Properties.review]
[Properties.review.passed]
subtasks = [\"notify stakeholders\"]
subtasks_allow_cancel = [true]
";
    let config = Config::from_str(input).unwrap();
    let prop = config.properties.get("review").unwrap();
    let branch = prop.branches.get("passed").unwrap();
    assert_eq!(branch.subtasks, vec!["notify stakeholders"]);
    assert_eq!(branch.subtasks_allow_cancel, vec![true]);
}

#[test]
fn mismatched_branch_subtasks_allow_cancel_length_is_an_error() {
    let input = "\
[Properties.review]
[Properties.review.passed]
subtasks = [\"notify stakeholders\", \"close ticket\"]
subtasks_allow_cancel = [true]
";
    let err = Config::from_str(input).unwrap_err();
    assert!(matches!(err, ConfigError::PropertyValidation { .. }));
}

#[test]
fn unknown_scalar_key_in_property_is_still_rejected_alongside_branches() {
    // `subtsaks` is a typo of `subtasks`; since it's a plain array (not a
    // table), it can't be mistaken for a branch sub-table either, so it must
    // still surface as a parse error, even though branches are now parsed via
    // `#[serde(flatten)]` instead of `deny_unknown_fields`.
    let input = "\
[Properties.feature]
subtsaks = [\"dev implementation\"]
";
    let result = Config::from_str(input);
    assert!(matches!(result, Err(ConfigError::Parse(_))));
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

// ── user identity (git_emails / git_names) ─────────────────────────────────────────

#[test]
fn user_without_git_emails_or_git_names_has_empty_vecs() {
    let config = Config::from_str("[Users.alice]\n").unwrap();
    let user = config.users.get("alice").unwrap();
    assert!(user.git_emails.is_empty());
    assert!(user.git_names.is_empty());
}

#[test]
fn user_with_git_emails_is_parsed() {
    let input = "\
[Users.alice]
git_emails = [\"alice@example.com\", \"a@example.org\"]
";
    let config = Config::from_str(input).unwrap();
    let user = config.users.get("alice").unwrap();
    assert_eq!(
        user.git_emails,
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
fn group_brief_and_description_are_parsed() {
    let file_content = "\
[Groups.devs]
brief = \"Product development team\"
description = \"Builds and maintains the product.\"
";
    let config = Config::from_str(file_content).unwrap();
    let group = config.groups.get("devs").unwrap();
    assert_eq!(group.brief.as_deref(), Some("Product development team"));
    assert_eq!(
        group.description.as_deref(),
        Some("Builds and maintains the product.")
    );
}

#[test]
fn group_without_brief_or_description_has_none() {
    let config = Config::from_str("[Groups.devs]\n").unwrap();
    let group = config.groups.get("devs").unwrap();
    assert_eq!(group.brief, None);
    assert_eq!(group.description, None);
}

#[test]
fn group_with_members_is_parsed() {
    let input = "\
[Users.alice]
[Users.bob]

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
git_emails = [\"alice@example.com\"]
git_names = [\"Alice\"]

[Groups.devs]
members = [\"alice\"]
";
    assert!(Config::from_str(input).is_ok());
}

// ── group member validation ────────────────────────────────────────────────────

#[test]
fn group_member_referencing_undefined_user_is_rejected() {
    let input = "\
[Groups.devs]
members = [\"ghost\"]
";
    let result = Config::from_str(input);
    match result {
        Err(ConfigError::GroupValidation { group, message }) => {
            assert_eq!(group, "devs");
            assert!(message.contains("ghost"), "message: {message:?}");
        }
        other => panic!("expected GroupValidation error, got {other:?}"),
    }
}

#[test]
fn group_member_referencing_defined_user_is_accepted() {
    let input = "\
[Users.alice]

[Groups.devs]
members = [\"alice\"]
";
    assert!(Config::from_str(input).is_ok());
}

#[test]
fn group_with_one_undefined_member_among_defined_ones_is_rejected() {
    let input = "\
[Users.alice]

[Groups.devs]
members = [\"alice\", \"ghost\"]
";
    let result = Config::from_str(input);
    assert!(matches!(result, Err(ConfigError::GroupValidation { .. })));
}

#[test]
fn multiple_groups_all_with_defined_members_are_accepted() {
    let input = "\
[Users.alice]
[Users.bob]

[Groups.devs]
members = [\"alice\"]

[Groups.qa]
members = [\"bob\", \"alice\"]
";
    assert!(Config::from_str(input).is_ok());
}

// ── duplicate identity validation ──────────────────────────────────────────

#[test]
fn duplicate_git_email_across_users_is_rejected() {
    let input = "\
[Users.alice]
git_emails = [\"shared@example.com\"]

[Users.bob]
git_emails = [\"shared@example.com\"]
";
    let result = Config::from_str(input);
    match result {
        Err(ConfigError::DuplicateIdentity {
            field,
            value,
            users,
        }) => {
            assert_eq!(field, "git_emails");
            assert_eq!(value, "shared@example.com");
            assert_eq!(users, vec!["alice".to_string(), "bob".to_string()]);
        }
        other => panic!("expected DuplicateIdentity error, got {other:?}"),
    }
}

#[test]
fn duplicate_git_name_across_users_is_rejected() {
    let input = "\
[Users.alice]
git_names = [\"Shared Name\"]

[Users.bob]
git_names = [\"Shared Name\"]
";
    let result = Config::from_str(input);
    match result {
        Err(ConfigError::DuplicateIdentity {
            field,
            value,
            users,
        }) => {
            assert_eq!(field, "git_names");
            assert_eq!(value, "Shared Name");
            assert_eq!(users, vec!["alice".to_string(), "bob".to_string()]);
        }
        other => panic!("expected DuplicateIdentity error, got {other:?}"),
    }
}

#[test]
fn distinct_git_emails_and_names_across_users_are_accepted() {
    let input = "\
[Users.alice]
git_emails = [\"alice@example.com\"]
git_names = [\"Alice\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
git_names = [\"Bob\"]
";
    assert!(Config::from_str(input).is_ok());
}

#[test]
fn same_user_listing_the_same_email_twice_is_not_a_duplicate() {
    let input = "\
[Users.alice]
git_emails = [\"alice@example.com\", \"alice@example.com\"]
";
    assert!(Config::from_str(input).is_ok());
}

// ── [General] section ───────────────────────────────────────────────────────

#[test]
fn general_section_defaults_to_warning_when_not_a_git_repo() {
    let config = Config::from_str("").unwrap();
    assert!(config.general.warn_when_not_a_git_repo);
}

#[test]
fn general_section_can_suppress_the_not_a_git_repo_warning() {
    let input = "\
[General]
warn_when_not_a_git_repo = false
";
    let config = Config::from_str(input).unwrap();
    assert!(!config.general.warn_when_not_a_git_repo);
}

#[test]
fn general_section_with_unknown_key_is_rejected() {
    let input = "\
[General]
bogus_key = true
";
    let result = Config::from_str(input);
    assert!(result.is_err());
}

// ── [Field.X] parsing ───────────────────────────────────────────────────────

#[test]
fn empty_toml_gives_empty_fields_and_forms() {
    let config = Config::from_str("").unwrap();
    assert!(config.fields.is_empty());
    assert!(config.forms.is_empty());
}

#[test]
fn field_with_all_keys_is_parsed() {
    let input = "\
[Field.gender]
label = \"Gender\"
default = \"(don't specify)\"
values = [\"male\", \"female\", \"other\"]
multiline = false
";
    let config = Config::from_str(input).unwrap();
    let field = config.fields.get("gender").unwrap();
    assert_eq!(field.name, "gender");
    assert_eq!(field.label, "Gender");
    assert_eq!(field.default.as_deref(), Some("(don't specify)"));
    assert_eq!(
        field.values,
        Some(vec![
            "male".to_string(),
            "female".to_string(),
            "other".to_string()
        ])
    );
    assert!(!field.multiline);
}

#[test]
fn field_without_label_falls_back_to_field_name() {
    let config = Config::from_str("[Field.gender]\n").unwrap();
    let field = config.fields.get("gender").unwrap();
    assert_eq!(field.label, "gender");
}

#[test]
fn field_without_optional_keys_has_sensible_defaults() {
    let config = Config::from_str("[Field.gender]\n").unwrap();
    let field = config.fields.get("gender").unwrap();
    assert_eq!(field.default, None);
    assert_eq!(field.validation_regex, None);
    assert_eq!(field.values, None);
    assert!(!field.multiline);
}

#[test]
fn field_with_validation_regex_is_parsed() {
    let input = "\
[Field.email]
validation_regex = \"^[^@]+@[^@]+$\"
";
    let config = Config::from_str(input).unwrap();
    let field = config.fields.get("email").unwrap();
    assert_eq!(field.validation_regex.as_deref(), Some("^[^@]+@[^@]+$"));
}

#[test]
fn field_with_multiline_true_is_parsed() {
    let input = "\
[Field.notes]
multiline = true
";
    let config = Config::from_str(input).unwrap();
    let field = config.fields.get("notes").unwrap();
    assert!(field.multiline);
}

#[test]
fn field_with_unknown_key_is_rejected() {
    let input = "\
[Field.gender]
bogus_key = true
";
    let result = Config::from_str(input);
    assert!(result.is_err());
}

#[test]
fn field_with_both_validation_regex_and_values_is_rejected() {
    let input = "\
[Field.gender]
validation_regex = \"^(male|female)$\"
values = [\"male\", \"female\"]
";
    let result = Config::from_str(input);
    match result {
        Err(ConfigError::FieldValidation { field, message }) => {
            assert_eq!(field, "gender");
            assert!(message.contains("validation_regex"), "message: {message:?}");
            assert!(message.contains("values"), "message: {message:?}");
        }
        other => panic!("expected FieldValidation error, got {other:?}"),
    }
}

#[test]
fn field_with_neither_validation_regex_nor_values_is_accepted() {
    let config = Config::from_str("[Field.notes]\n").unwrap();
    assert!(config.fields.contains_key("notes"));
}

#[test]
fn field_with_invalid_validation_regex_is_rejected() {
    let input = "\
[Field.email]
validation_regex = \"[\"
";
    let result = Config::from_str(input);
    match result {
        Err(ConfigError::FieldValidation { field, message }) => {
            assert_eq!(field, "email");
            assert!(message.contains("regex"), "message: {message:?}");
        }
        other => panic!("expected FieldValidation error, got {other:?}"),
    }
}

#[test]
fn field_with_multiline_default_but_multiline_false_is_rejected() {
    let input = "\
[Field.bio]
default = \"\"\"
line one
line two\"\"\"
";
    let result = Config::from_str(input);
    match result {
        Err(ConfigError::FieldValidation { field, message }) => {
            assert_eq!(field, "bio");
            assert!(message.contains("newline"), "message: {message:?}");
            assert!(message.contains("multiline"), "message: {message:?}");
        }
        other => panic!("expected FieldValidation error, got {other:?}"),
    }
}

#[test]
fn field_with_multiline_default_and_multiline_true_is_accepted() {
    let input = "\
[Field.bio]
multiline = true
default = \"\"\"
line one
line two\"\"\"
";
    let config = Config::from_str(input).unwrap();
    let field = config.fields.get("bio").unwrap();
    assert_eq!(field.default.as_deref(), Some("line one\nline two"));
}

#[test]
fn field_with_single_line_default_and_multiline_false_is_accepted() {
    let input = "\
[Field.email]
default = \"placeholder\"
";
    let config = Config::from_str(input).unwrap();
    let field = config.fields.get("email").unwrap();
    assert_eq!(field.default.as_deref(), Some("placeholder"));
}

// ── [Form.X] parsing ────────────────────────────────────────────────────────

#[test]
fn form_with_fields_and_validation_is_parsed() {
    let input = "\
[Field.first_name]
[Field.last_name]

[Form.personal_info]
fields = [\"first_name\", \"last_name\"]
validation = \"scripts/validate_personal_info.sh\"
";
    let config = Config::from_str(input).unwrap();
    let form = config.forms.get("personal_info").unwrap();
    assert_eq!(form.name, "personal_info");
    assert_eq!(form.fields, vec!["first_name", "last_name"]);
    assert_eq!(
        form.validation.as_deref(),
        Some("scripts/validate_personal_info.sh")
    );
}

#[test]
fn form_without_validation_has_none() {
    let input = "\
[Field.first_name]

[Form.personal_info]
fields = [\"first_name\"]
";
    let config = Config::from_str(input).unwrap();
    let form = config.forms.get("personal_info").unwrap();
    assert_eq!(form.validation, None);
}

#[test]
fn form_without_fields_has_empty_vec() {
    let config = Config::from_str("[Form.empty_form]\n").unwrap();
    let form = config.forms.get("empty_form").unwrap();
    assert!(form.fields.is_empty());
}

#[test]
fn form_with_unknown_key_is_rejected() {
    let input = "\
[Form.personal_info]
bogus_key = true
";
    let result = Config::from_str(input);
    assert!(result.is_err());
}

#[test]
fn form_field_referencing_undefined_field_is_rejected() {
    let input = "\
[Form.personal_info]
fields = [\"ghost\"]
";
    let result = Config::from_str(input);
    match result {
        Err(ConfigError::FormValidation { form, message }) => {
            assert_eq!(form, "personal_info");
            assert!(message.contains("ghost"), "message: {message:?}");
        }
        other => panic!("expected FormValidation error, got {other:?}"),
    }
}

#[test]
fn form_with_duplicate_field_names_is_rejected() {
    let input = "\
[Field.first_name]

[Form.personal_info]
fields = [\"first_name\", \"first_name\"]
";
    let result = Config::from_str(input);
    match result {
        Err(ConfigError::FormValidation { form, message }) => {
            assert_eq!(form, "personal_info");
            assert!(message.contains("first_name"), "message: {message:?}");
        }
        other => panic!("expected FormValidation error, got {other:?}"),
    }
}

// ── `form` key on [Properties.X] ─────────────────────────────────────────────

#[test]
fn property_without_form_key_has_none() {
    let config = Config::from_str("[Properties.bug]\n").unwrap();
    let prop = config.properties.get("bug").unwrap();
    assert_eq!(prop.form, None);
}

#[test]
fn property_form_referencing_defined_form_is_accepted() {
    let input = "\
[Form.personal_info]

[Properties.customer]
form = \"personal_info\"
";
    let config = Config::from_str(input).unwrap();
    let prop = config.properties.get("customer").unwrap();
    assert_eq!(prop.form.as_deref(), Some("personal_info"));
}

#[test]
fn property_form_referencing_undefined_form_is_rejected() {
    let input = "\
[Properties.customer]
form = \"ghost_form\"
";
    let result = Config::from_str(input);
    match result {
        Err(ConfigError::PropertyValidation { property, message }) => {
            assert_eq!(property, "customer");
            assert!(message.contains("ghost_form"), "message: {message:?}");
        }
        other => panic!("expected PropertyValidation error, got {other:?}"),
    }
}
