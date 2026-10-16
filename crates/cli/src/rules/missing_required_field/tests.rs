use super::*;
use crate::config::{Config, FieldConfig, FormConfig, PropertyConfig};
use crate::parser::{FileItem, parse};
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

/// Builds a `Config` with one property (`onboarding`) carrying a form
/// (`personal_info`) whose fields are `first_name` (label "First Name") and
/// `email` (label "Email").
fn config_with_onboarding_form() -> Config {
    let mut fields = std::collections::HashMap::new();
    fields.insert(
        "first_name".to_string(),
        FieldConfig {
            name: "first_name".to_string(),
            label: "First Name".to_string(),
            default: None,
            validation_regex: None,
            values: None,
            multiline: false,
        },
    );
    fields.insert(
        "email".to_string(),
        FieldConfig {
            name: "email".to_string(),
            label: "Email".to_string(),
            default: None,
            validation_regex: None,
            values: None,
            multiline: false,
        },
    );

    let mut forms = std::collections::HashMap::new();
    forms.insert(
        "personal_info".to_string(),
        FormConfig {
            name: "personal_info".to_string(),
            fields: vec!["first_name".to_string(), "email".to_string()],
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
fn flags_a_task_missing_every_form_field() {
    let input = "\
- [ ] new colleague: John #onboarding
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = missing_required_field(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::MissingRequiredField);
    assert_eq!(
        issues[0].data,
        Some(IssueData::MissingRequiredField {
            missing: vec!["First Name".to_string(), "Email".to_string()]
        })
    );
}

#[test]
fn flags_only_the_fields_that_are_absent() {
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ John
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = missing_required_field(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::MissingRequiredField {
            missing: vec!["Email".to_string()]
        })
    );
}

#[test]
fn no_issue_when_every_form_field_is_present() {
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ John
  __Email:__ john.doe@gmail.com
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = missing_required_field(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn no_issue_for_a_property_with_no_form() {
    let input = "\
- [ ] a plain task #onboarding
";
    let items = p(input);
    let mut config = config_with_onboarding_form();
    config.properties.get_mut("onboarding").unwrap().form = None;
    let issues = missing_required_field(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn no_issue_when_form_reference_is_undefined() {
    // Undefined `form = "..."` reference is a separate config-validation
    // check's concern (not this rule's).
    let input = "\
- [ ] new colleague: John #onboarding
";
    let items = p(input);
    let mut config = config_with_onboarding_form();
    config.forms.clear();
    let issues = missing_required_field(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn field_present_on_subtask_is_checked_independently_of_its_parent() {
    let input = "\
- [ ] parent
  - [ ] sub #onboarding
    __First Name:__ John
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = missing_required_field(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::MissingRequiredField {
            missing: vec!["Email".to_string()]
        })
    );
}
