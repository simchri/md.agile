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
            default_is_valid_entry: false,
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
            default_is_valid_entry: false,
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
- [x] new colleague: John #onboarding
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
            label: "First Name".to_string()
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
- [x] new colleague: John #onboarding
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
            label: "Gender".to_string()
        })
    );
}

#[test]
fn no_issue_when_every_present_field_has_a_real_value() {
    let input = "\
- [x] new colleague: John #onboarding
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
- [x] new colleague: John #onboarding
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
- [x] a plain task #onboarding
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
- [x] new colleague: John #onboarding
  __First Name:__ ___
";
    let items = p(input);
    let mut config = config_with_onboarding_form();
    config.forms.clear();
    let issues = field_left_unfilled(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn reports_one_issue_per_unfilled_field_anchored_at_its_own_line() {
    let input = "\
- [x] new colleague: John #onboarding
  __First Name:__ ___
  __Gender:__ (don't specify)
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = field_left_unfilled(&items, &config);

    assert_eq!(issues.len(), 2);
    assert_eq!(
        issues[0].data,
        Some(IssueData::FieldLeftUnfilled {
            label: "First Name".to_string()
        })
    );
    // Line 1 is the task; line 2 is `__First Name:__ ___`.
    assert_eq!(issues[0].location.line, 2);
    assert_eq!(
        issues[1].data,
        Some(IssueData::FieldLeftUnfilled {
            label: "Gender".to_string()
        })
    );
    // Line 3 is `__Gender:__ (don't specify)`.
    assert_eq!(issues[1].location.line, 3);
}

#[test]
fn field_present_on_subtask_is_checked_independently_of_its_parent() {
    let input = "\
- [ ] parent
  - [x] sub #onboarding
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
            label: "First Name".to_string()
        })
    );
}

#[test]
fn a_node_with_two_form_carrying_properties_gets_one_issue_per_property_per_field() {
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
- [x] new colleague: John #onboarding #relocation
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
                label: "First Name".to_string()
            })
        );
    }
}

#[test]
fn no_issue_for_a_todo_task_with_an_unfilled_field() {
    // This check only matters at the moment a task is (about to be) marked
    // done -- see `check_node`/`check_completable` below for the
    // preventive half of this rule. The full-project scan here stays quiet
    // for a still-open task, same as E004 (`incomplete_parent`).
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ ___
  __Gender:__ male
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = field_left_unfilled(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn check_node_flags_an_unfilled_field_regardless_of_status() {
    // `check_node` is the unconditional half of this rule, reused by
    // `check_completable` to validate a node *before* it's marked done --
    // it doesn't look at the node's current status at all.
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ ___
  __Gender:__ male
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let node = crate::rules::find_node_by_line(&items, 1).unwrap();
    let issues = check_node(node.markers(), node.fields(), &config);

    assert_eq!(
        issues.into_iter().map(|i| i.data).collect::<Vec<_>>(),
        vec![Some(IssueData::FieldLeftUnfilled {
            label: "First Name".to_string()
        })]
    );
}

#[test]
fn no_issue_when_field_opts_in_to_default_is_valid_entry() {
    // "Gender"'s custom default "(don't specify)" is itself a legitimate,
    // deliberately chosen answer (one of the field's allowed `values`) when
    // `default_is_valid_entry = true` -- it must never be flagged as
    // unfilled, even if the task is done and the value still equals that
    // default exactly.
    let mut config = config_with_onboarding_form();
    config
        .fields
        .get_mut("gender")
        .unwrap()
        .default_is_valid_entry = true;
    let input = "\
- [x] new colleague: John #onboarding
  __First Name:__ John
  __Gender:__ (don't specify)
";
    let items = p(input);
    let issues = field_left_unfilled(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn default_is_valid_entry_true_also_suppresses_the_plain_underscore_placeholder() {
    // Per the confirmed design decision, the flag governs the field's
    // *resolved* placeholder -- the plain generic `___` when the field has
    // no custom `default` at all -- exactly the same as a custom default.
    let mut config = config_with_onboarding_form();
    config
        .fields
        .get_mut("first_name")
        .unwrap()
        .default_is_valid_entry = true;
    let input = "\
- [x] new colleague: John #onboarding
  __First Name:__ ___
  __Gender:__ male
";
    let items = p(input);
    let issues = field_left_unfilled(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn default_is_valid_entry_true_does_not_suppress_a_truly_blank_value() {
    // `default_is_valid_entry` only means the *placeholder itself* (custom
    // default, or plain `___`) counts as a deliberately chosen answer -- it
    // does not disable the check altogether. A field left entirely blank
    // (no value at all after the label) is never a legitimate answer and
    // must still be flagged, even when the field opts in.
    let mut config = config_with_onboarding_form();
    config
        .fields
        .get_mut("gender")
        .unwrap()
        .default_is_valid_entry = true;
    let input = "\
- [x] new colleague: John #onboarding
  __First Name:__ John
  __Gender:__ 
";
    let items = p(input);
    let issues = field_left_unfilled(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::FieldLeftUnfilled {
            label: "Gender".to_string()
        })
    );
}

#[test]
fn a_truly_blank_value_is_flagged_even_without_default_is_valid_entry() {
    // Same blank-value detection applies in the default (opted-out) case
    // too: a field whose value is blank, not the literal resolved
    // placeholder text, must still be caught as unfilled.
    let input = "\
- [x] new colleague: John #onboarding
  __First Name:__ 
  __Gender:__ male
";
    let items = p(input);
    let config = config_with_onboarding_form();
    let issues = field_left_unfilled(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::FieldLeftUnfilled {
            label: "First Name".to_string()
        })
    );
}

#[test]
fn placeholder_value_with_incidental_trailing_whitespace_is_still_flagged() {
    // A value that's the resolved placeholder plus stray trailing
    // whitespace (easy to introduce by accident when editing the body
    // line) is just as "unfilled" as the bare placeholder -- it must not
    // slip through just because it isn't byte-for-byte equal to the
    // placeholder text.
    let input = "\
- [x] new colleague: John #onboarding
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
            label: "First Name".to_string()
        })
    );
}

#[test]
fn flags_a_multiline_field_whose_captured_content_is_empty() {
    let mut fields = std::collections::HashMap::new();
    fields.insert(
        "bio".to_string(),
        FieldConfig {
            name: "bio".to_string(),
            label: "Bio".to_string(),
            default: None,
            validation_regex: None,
            values: None,
            multiline: true,
            default_is_valid_entry: false,
        },
    );
    let mut forms = std::collections::HashMap::new();
    forms.insert(
        "personal_info".to_string(),
        FormConfig {
            name: "personal_info".to_string(),
            fields: vec!["bio".to_string()],
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
- [x] new colleague: John #onboarding
  __Bio:__
  ...
";
    let items = p(input);
    let issues = field_left_unfilled(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::FieldLeftUnfilled {
            label: "Bio".to_string()
        })
    );
}
