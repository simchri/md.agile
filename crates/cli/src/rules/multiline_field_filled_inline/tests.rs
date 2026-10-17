use super::*;
use crate::config::{Config, FieldConfig, FormConfig, PropertyConfig};
use crate::parser::{FileItem, parse};
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

/// Builds a `Config` with one property (`onboarding`) carrying a form
/// (`personal_info`) whose only field is `bio` (label "Bio"), configured as
/// multiline.
fn config_with_multiline_bio_field() -> Config {
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
            help: None,
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

    Config {
        properties,
        fields,
        forms,
        ..Config::default()
    }
}

#[test]
fn flags_a_multiline_field_written_inline() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__ Grew up in Portland.
";
    let items = p(input);
    let config = config_with_multiline_bio_field();
    let issues = multiline_field_filled_inline(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::MultilineFieldFilledInline);
    assert_eq!(
        issues[0].data,
        Some(IssueData::MultilineFieldFilledInline {
            label: "Bio".to_string()
        })
    );
}

#[test]
fn flags_the_field_line_itself_not_the_task_line() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__ Grew up in Portland.
";
    let items = p(input);
    let config = config_with_multiline_bio_field();
    let issues = multiline_field_filled_inline(&items, &config);

    assert_eq!(issues.len(), 1);
    // Line 1 is the task; line 2 is the `__Bio:__ ...` field line -- the
    // issue must be anchored there, not at the task that merely carries
    // the property.
    assert_eq!(issues[0].location.line, 2);
}

#[test]
fn does_not_flag_a_multiline_field_written_properly_across_lines() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  Grew up in Portland.
  ...
";
    let items = p(input);
    let config = config_with_multiline_bio_field();
    let issues = multiline_field_filled_inline(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_a_multiline_field_written_properly_with_no_closing_marker() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  Grew up in Portland.
";
    let items = p(input);
    let config = config_with_multiline_bio_field();
    let issues = multiline_field_filled_inline(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_an_empty_multiline_field_written_properly() {
    // Empty content is E026's concern (field_left_unfilled), not this
    // rule's -- the bare `__Bio:__` opening line is still the correct
    // (multiline) grammar.
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  ...
";
    let items = p(input);
    let config = config_with_multiline_bio_field();
    let issues = multiline_field_filled_inline(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn flags_a_multiline_field_written_inline_with_only_trailing_whitespace() {
    // `__Bio:__ ` (one trailing space, nothing else) still matches the
    // single-line grammar, not the bare multiline-opening grammar -- see
    // `crate::parser::parse_single_line_field` -- so it's flagged too, even
    // though its value happens to be empty.
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__ 
";
    let items = p(input);
    let config = config_with_multiline_bio_field();
    let issues = multiline_field_filled_inline(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::MultilineFieldFilledInline {
            label: "Bio".to_string()
        })
    );
}

#[test]
fn does_not_flag_a_single_line_configured_field_written_inline() {
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
  __First Name:__ John
";
    let items = p(input);
    let issues = multiline_field_filled_inline(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_a_field_that_is_entirely_absent() {
    // Absence is E024's concern (missing_required_field), not this rule's.
    let input = "\
- [ ] new colleague: John #onboarding
";
    let items = p(input);
    let config = config_with_multiline_bio_field();
    let issues = multiline_field_filled_inline(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_a_malformed_field_line() {
    // Missing the required space is E025's concern
    // (missing_space_after_field_label), not this rule's -- a malformed
    // line has no parsed value at all, so it can't be checked here either.
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__Grew up in Portland.
";
    let items = p(input);
    let config = config_with_multiline_bio_field();
    let issues = multiline_field_filled_inline(&items, &config);

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
    let issues = multiline_field_filled_inline(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn this_only_matters_regardless_of_done_status() {
    // Unlike E026, this is a structural/syntax mistake, independent of
    // whether the task is open or done -- it's wrong the moment it's
    // written, same as E024/E025/E027.
    let input = "\
- [x] new colleague: John #onboarding
  __Bio:__ Grew up in Portland.
";
    let items = p(input);
    let config = config_with_multiline_bio_field();
    let issues = multiline_field_filled_inline(&items, &config);

    assert_eq!(issues.len(), 1);
}

#[test]
fn each_node_in_the_tree_is_checked_independently() {
    let input = "\
- [ ] parent
  - [ ] sub #onboarding
    __Bio:__ Grew up in Portland.
";
    let items = p(input);
    let config = config_with_multiline_bio_field();
    let issues = multiline_field_filled_inline(&items, &config);

    assert_eq!(issues.len(), 1);
}
