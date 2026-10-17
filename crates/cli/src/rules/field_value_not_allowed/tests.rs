use super::*;
use crate::config::{Config, FieldConfig, FormConfig, PropertyConfig};
use crate::parser::{FileItem, parse};
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

/// Builds a `Config` with one property (`onboarding`) carrying a form
/// (`personal_info`) whose only field is `gender` (label "Gender"),
/// restricted to `values = ["male", "female", "(don't specify)"]` with a
/// custom default of `"(don't specify)"` (itself one of the allowed
/// values, per the README.vision.md example).
fn config_with_gender_field() -> Config {
    let mut fields = std::collections::HashMap::new();
    fields.insert(
        "gender".to_string(),
        FieldConfig {
            name: "gender".to_string(),
            label: "Gender".to_string(),
            default: Some("(don't specify)".to_string()),
            values: Some(vec![
                "male".to_string(),
                "female".to_string(),
                "(don't specify)".to_string(),
            ]),
            multiline: false,
            help: None,
            default_is_valid_entry: false,
        },
    );

    let mut forms = std::collections::HashMap::new();
    forms.insert(
        "personal_info".to_string(),
        FormConfig {
            name: "personal_info".to_string(),
            fields: vec!["gender".to_string()],
            validation: None,
        },
    );

    let mut properties = std::collections::HashMap::new();
    properties.insert(
        "onboarding".to_string(),
        PropertyConfig {
            name: "onboarding".to_string(),
            brief: None,
            description: None,
            subtasks: vec![],
            subtasks_allow_cancel: vec![],
            neighbortasks: vec![],
            form: Some("personal_info".to_string()),
            branches: Default::default(),
        },
    );

    Config {
        properties,
        fields,
        forms,
        ..Config::default()
    }
}

#[test]
fn flags_a_value_outside_the_allowed_set() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Gender:__ nonbinary
";
    let items = p(input);
    let config = config_with_gender_field();
    let issues = field_value_not_allowed(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::FieldValueNotAllowed);
    assert_eq!(
        issues[0].data,
        Some(IssueData::FieldValueNotAllowed {
            label: "Gender".to_string(),
            value: "nonbinary".to_string(),
            allowed: vec![
                "male".to_string(),
                "female".to_string(),
                "(don't specify)".to_string()
            ],
        })
    );
}

#[test]
fn flags_the_field_line_itself_not_the_task_line() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Gender:__ nonbinary
";
    let items = p(input);
    let config = config_with_gender_field();
    let issues = field_value_not_allowed(&items, &config);

    assert_eq!(issues.len(), 1);
    // Line 1 is the task; line 2 is the `__Gender:__ ...` field line -- the
    // issue must be anchored there.
    assert_eq!(issues[0].location.line, 2);
}

#[test]
fn does_not_flag_a_value_that_is_in_the_allowed_set() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Gender:__ male
";
    let items = p(input);
    let config = config_with_gender_field();
    let issues = field_value_not_allowed(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_a_field_still_at_its_custom_default_placeholder() {
    // "(don't specify)" is both the configured default and itself one of
    // the allowed values here, but even if it weren't, a field still at
    // its unfilled placeholder is E026's concern, not this rule's -- this
    // check only judges an actual, deliberately entered value.
    let input = "\
- [ ] new colleague: John #onboarding
  __Gender:__ (don't specify)
";
    let items = p(input);
    let config = config_with_gender_field();
    let issues = field_value_not_allowed(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_a_blank_value() {
    // Blank is E026's concern (field_left_unfilled), not this rule's --
    // there's no entered value to validate against the allowed set.
    let input = "\
- [ ] new colleague: John #onboarding
  __Gender:__ 
";
    let items = p(input);
    let config = config_with_gender_field();
    let issues = field_value_not_allowed(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_a_field_with_no_configured_values() {
    let mut fields = std::collections::HashMap::new();
    fields.insert(
        "first_name".to_string(),
        FieldConfig {
            name: "first_name".to_string(),
            label: "First Name".to_string(),
            default: None,
            values: None,
            multiline: false,
            help: None,
            default_is_valid_entry: false,
        },
    );
    let mut forms = std::collections::HashMap::new();
    forms.insert(
        "personal_info".to_string(),
        FormConfig {
            name: "personal_info".to_string(),
            fields: vec!["first_name".to_string()],
            validation: None,
        },
    );
    let mut properties = std::collections::HashMap::new();
    properties.insert(
        "onboarding".to_string(),
        PropertyConfig {
            name: "onboarding".to_string(),
            brief: None,
            description: None,
            subtasks: vec![],
            subtasks_allow_cancel: vec![],
            neighbortasks: vec![],
            form: Some("personal_info".to_string()),
            branches: Default::default(),
        },
    );
    let config = Config {
        properties,
        fields,
        forms,
        ..Config::default()
    };

    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ anything at all
";
    let items = p(input);
    let issues = field_value_not_allowed(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_a_field_that_is_entirely_absent() {
    // Absence is E024's concern (missing_required_field), not this rule's.
    let input = "\
- [ ] new colleague: John #onboarding
";
    let items = p(input);
    let config = config_with_gender_field();
    let issues = field_value_not_allowed(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_a_malformed_field_line() {
    // Missing the required space is E025's concern
    // (missing_space_after_field_label), not this rule's -- a malformed
    // line has no parsed value at all, so it can't be checked here either.
    let input = "\
- [ ] new colleague: John #onboarding
  __Gender:__nonbinary
";
    let items = p(input);
    let config = config_with_gender_field();
    let issues = field_value_not_allowed(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_a_property_without_a_form() {
    let mut properties = std::collections::HashMap::new();
    properties.insert(
        "plain".to_string(),
        PropertyConfig {
            name: "plain".to_string(),
            brief: None,
            description: None,
            subtasks: vec![],
            subtasks_allow_cancel: vec![],
            neighbortasks: vec![],
            form: None,
            branches: Default::default(),
        },
    );
    let config = Config {
        properties,
        ..Config::default()
    };

    let input = "\
- [ ] a plain task #plain
";
    let items = p(input);
    let issues = field_value_not_allowed(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn this_only_matters_regardless_of_done_status() {
    // Like E028, this is a validity mistake independent of whether the
    // task is open or done -- it's wrong the moment it's written.
    let input = "\
- [x] new colleague: John #onboarding
  __Gender:__ nonbinary
";
    let items = p(input);
    let config = config_with_gender_field();
    let issues = field_value_not_allowed(&items, &config);

    assert_eq!(issues.len(), 1);
}

#[test]
fn each_node_in_the_tree_is_checked_independently() {
    let input = "\
- [ ] parent
  - [ ] sub #onboarding
    __Gender:__ nonbinary
";
    let items = p(input);
    let config = config_with_gender_field();
    let issues = field_value_not_allowed(&items, &config);

    assert_eq!(issues.len(), 1);
}
