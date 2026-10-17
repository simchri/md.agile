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
            values: None,
            multiline: false,
            help: None,
            default_is_valid_entry: false,
        },
    );
    fields.insert(
        "email".to_string(),
        FieldConfig {
            name: "email".to_string(),
            label: "Email".to_string(),
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
fn a_node_with_two_form_carrying_properties_gets_one_issue_per_property() {
    // Assumption worth flagging: issues are computed per *marker*, not
    // bundled across the whole node. A node tagged with two different
    // properties that each carry a form produces two separate `Issue`s
    // (each bundling that property's own missing fields), not one combined
    // issue for the node.
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
";
    let items = p(input);
    let issues = missing_required_field(&items, &config);

    assert_eq!(issues.len(), 2);
    for issue in &issues {
        assert_eq!(
            issue.data,
            Some(IssueData::MissingRequiredField {
                missing: vec!["First Name".to_string(), "Email".to_string()]
            })
        );
    }
}

#[test]
fn form_field_name_undefined_in_config_fields_is_silently_not_checked() {
    // Assumption worth flagging: a `[Form.X].fields` entry that doesn't
    // resolve to a defined `[Field.X]` is filtered out via `filter_map`
    // rather than being reported as "missing" -- it's deferred entirely to
    // the separate "form's fields reference existing [Field.X]" config
    // check, so it never shows up in this rule's output even though the
    // task body has no line for it at all.
    let mut config = config_with_onboarding_form();
    config
        .forms
        .get_mut("personal_info")
        .unwrap()
        .fields
        .push("phone_number".to_string()); // not present in config.fields
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ John
  __Email:__ john.doe@gmail.com
";
    let items = p(input);
    let issues = missing_required_field(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn label_matching_is_case_sensitive() {
    // A field line whose label differs only in case from the configured
    // label does not satisfy the requirement -- it's still reported
    // missing.
    let input = "\
- [ ] new colleague: John #onboarding
  __first name:__ John
  __Email:__ john.doe@gmail.com
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = missing_required_field(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::MissingRequiredField {
            missing: vec!["First Name".to_string()]
        })
    );
}

#[test]
fn two_fields_sharing_the_same_label_let_one_field_line_satisfy_both() {
    // Assumption worth flagging: presence is matched purely by label text,
    // not by the `[Field.X]` key. If two distinct fields in config happen
    // to share the same `label` (nothing prevents this), one matching body
    // line satisfies both -- the second field is never reported missing
    // even though its own distinct `[Field.X]` entry has no corresponding
    // line.
    let mut config = config_with_onboarding_form();
    config.fields.insert(
        "preferred_name".to_string(),
        FieldConfig {
            name: "preferred_name".to_string(),
            label: "First Name".to_string(), // duplicate label, different field
            default: None,
            values: None,
            multiline: false,
            help: None,
            default_is_valid_entry: false,
        },
    );
    config
        .forms
        .get_mut("personal_info")
        .unwrap()
        .fields
        .push("preferred_name".to_string());

    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ John
  __Email:__ john.doe@gmail.com
";
    let items = p(input);
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

#[test]
fn label_with_a_malformed_field_line_is_excluded_from_missing_and_not_double_reported() {
    // "Email" has a field line, just a malformed one (missing space after
    // the label) -- E025 (missing_space_after_field_label) is responsible
    // for flagging that; E024 must not also list "Email" as missing.
    let input = "\
- [ ] new colleague: John #onboarding
  __Email:__john@example.com
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = missing_required_field(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::MissingRequiredField {
            missing: vec!["First Name".to_string()]
        })
    );
}

#[test]
fn all_labels_malformed_reports_no_e024_issue_at_all() {
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__John
  __Email:__john@example.com
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = missing_required_field(&items, &config);

    assert_eq!(issues.len(), 0);
}
