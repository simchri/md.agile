use super::*;
use crate::config::{Config, FieldConfig, FormConfig, PropertyConfig};
use crate::parser::{FileItem, parse};
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

/// Builds a `Config` with one property (`onboarding`) carrying a form
/// (`personal_info`) whose fields are `first_name` (label "First Name", no
/// custom default -- plain `___` placeholder) and `gender` (label "Gender",
/// custom default `"(don't specify)"`).
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
        "gender".to_string(),
        FieldConfig {
            name: "gender".to_string(),
            label: "Gender".to_string(),
            default: Some("(don't specify)".to_string()),
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
            fields: vec!["first_name".to_string(), "gender".to_string()],
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
fn flags_a_field_still_at_the_plain_underscore_placeholder() {
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ ___
  __Gender:__ male
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = field_left_unfilled(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::FieldLeftUnfilled);
    assert_eq!(
        issues[0].data,
        Some(IssueData::FieldLeftUnfilled {
            unfilled: vec!["First Name".to_string()]
        })
    );
}

#[test]
fn flags_a_field_still_at_its_custom_default() {
    // "Gender" has a configured custom default of "(don't specify)" -- per
    // the confirmed design decision, a field that still carries exactly
    // that resolved placeholder counts as unfilled, same as the plain
    // `___` case.
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ John
  __Gender:__ (don't specify)
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = field_left_unfilled(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::FieldLeftUnfilled {
            unfilled: vec!["Gender".to_string()]
        })
    );
}

#[test]
fn no_issue_when_every_present_field_has_a_real_value() {
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ John
  __Gender:__ male
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = field_left_unfilled(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn no_issue_for_a_field_that_is_entirely_absent() {
    // Absence is E024's concern (missing_required_field), not this rule's.
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ John
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = field_left_unfilled(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn no_issue_for_a_property_with_no_form() {
    let input = "\
- [ ] a plain task #onboarding
  __First Name:__ ___
";
    let items = p(input);
    let mut config = config_with_onboarding_form();
    config.properties.get_mut("onboarding").unwrap().form = None;
    let issues = field_left_unfilled(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn no_issue_when_form_reference_is_undefined() {
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ ___
";
    let items = p(input);
    let mut config = config_with_onboarding_form();
    config.forms.clear();
    let issues = field_left_unfilled(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn reports_multiple_unfilled_fields_in_one_issue() {
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ ___
  __Gender:__ (don't specify)
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = field_left_unfilled(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::FieldLeftUnfilled {
            unfilled: vec!["First Name".to_string(), "Gender".to_string()]
        })
    );
}

#[test]
fn field_present_on_subtask_is_checked_independently_of_its_parent() {
    let input = "\
- [ ] parent
  - [ ] sub #onboarding
    __First Name:__ ___
    __Gender:__ male
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = field_left_unfilled(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::FieldLeftUnfilled {
            unfilled: vec!["First Name".to_string()]
        })
    );
}

#[test]
fn a_node_with_two_form_carrying_properties_gets_one_issue_per_property() {
    let mut config = config_with_onboarding_form();
    config.properties.insert(
        "relocation".to_string(),
        PropertyConfig {
            name: "relocation".to_string(),
            brief: None,
            description: None,
            subtasks: vec![],
            subtasks_allow_cancel: vec![],
            neighbortasks: vec![],
            form: Some("personal_info".to_string()),
            branches: Default::default(),
        },
    );
    let input = "\
- [ ] new colleague: John #onboarding #relocation
  __First Name:__ ___
  __Gender:__ male
";
    let items = p(input);
    let issues = field_left_unfilled(&items, &config);

    assert_eq!(issues.len(), 2);
    for issue in &issues {
        assert_eq!(
            issue.data,
            Some(IssueData::FieldLeftUnfilled {
                unfilled: vec!["First Name".to_string()]
            })
        );
    }
}
