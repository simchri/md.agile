use super::*;
use crate::config::{Config, FieldConfig, FormConfig, PropertyConfig};
use crate::parser::{FileItem, parse};
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

fn field(label: &str) -> FieldConfig {
    FieldConfig {
        name: label.to_lowercase(),
        label: label.to_string(),
        default: None,
        validation_regex: None,
        values: None,
        multiline: false,
        help: None,
        default_is_valid_entry: false,
    }
}

fn property(form: &str) -> PropertyConfig {
    PropertyConfig {
        name: String::new(),
        brief: None,
        description: None,
        subtasks: vec![],
        subtasks_allow_cancel: vec![],
        neighbortasks: vec![],
        form: Some(form.to_string()),
        branches: Default::default(),
    }
}

/// Builds a `Config` with two properties, each attaching its own,
/// otherwise-unrelated form, both of which happen to declare a field
/// labeled "Name":
/// - `propertyA` -> form `form_a` -> field `name_a` (label "Name")
/// - `propertyB` -> form `form_b` -> field `name_b` (label "Name")
fn config_with_colliding_forms() -> Config {
    let mut fields = std::collections::HashMap::new();
    fields.insert("name_a".to_string(), field("Name"));
    fields.insert("name_b".to_string(), field("Name"));

    let mut forms = std::collections::HashMap::new();
    forms.insert(
        "form_a".to_string(),
        FormConfig {
            name: "form_a".to_string(),
            fields: vec!["name_a".to_string()],
            validation: None,
        },
    );
    forms.insert(
        "form_b".to_string(),
        FormConfig {
            name: "form_b".to_string(),
            fields: vec!["name_b".to_string()],
            validation: None,
        },
    );

    let mut properties = std::collections::HashMap::new();
    properties.insert("propertyA".to_string(), property("form_a"));
    properties.insert("propertyB".to_string(), property("form_b"));

    Config {
        properties,
        fields,
        forms,
        ..Config::default()
    }
}

#[test]
fn flags_two_properties_whose_forms_share_a_field_label() {
    let input = "\
- [ ] new colleague: John #propertyA #propertyB
  __Name:__ John
";
    let items = p(input);
    let config = config_with_colliding_forms();
    let issues = colliding_field_labels(&items, &config);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::CollidingFieldLabels);
    assert_eq!(
        issues[0].data,
        Some(IssueData::CollidingFieldLabels {
            label: "Name".to_string(),
            properties: vec!["propertyA".to_string(), "propertyB".to_string()],
        })
    );
}

#[test]
fn does_not_flag_a_single_property_with_a_form() {
    let input = "\
- [ ] new colleague: John #propertyA
  __Name:__ John
";
    let items = p(input);
    let config = config_with_colliding_forms();
    let issues = colliding_field_labels(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn does_not_flag_two_properties_whose_forms_have_distinct_labels() {
    let mut fields = std::collections::HashMap::new();
    fields.insert("name_a".to_string(), field("Name"));
    fields.insert("device".to_string(), field("Device"));

    let mut forms = std::collections::HashMap::new();
    forms.insert(
        "form_a".to_string(),
        FormConfig {
            name: "form_a".to_string(),
            fields: vec!["name_a".to_string()],
            validation: None,
        },
    );
    forms.insert(
        "form_b".to_string(),
        FormConfig {
            name: "form_b".to_string(),
            fields: vec!["device".to_string()],
            validation: None,
        },
    );

    let mut properties = std::collections::HashMap::new();
    properties.insert("propertyA".to_string(), property("form_a"));
    properties.insert("propertyB".to_string(), property("form_b"));

    let config = Config {
        properties,
        fields,
        forms,
        ..Config::default()
    };

    let input = "\
- [ ] new colleague: John #propertyA #propertyB
  __Name:__ John
  __Device:__ laptop
";
    let items = p(input);
    let issues = colliding_field_labels(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn two_properties_sharing_the_same_form_are_not_a_collision() {
    // Both properties attach the *same* form -- there's only one set of
    // field labels involved, not two forms disagreeing about what "Name"
    // means, so this isn't the collision this rule is about.
    let mut fields = std::collections::HashMap::new();
    fields.insert("name_a".to_string(), field("Name"));

    let mut forms = std::collections::HashMap::new();
    forms.insert(
        "form_a".to_string(),
        FormConfig {
            name: "form_a".to_string(),
            fields: vec!["name_a".to_string()],
            validation: None,
        },
    );

    let mut properties = std::collections::HashMap::new();
    properties.insert("propertyA".to_string(), property("form_a"));
    properties.insert("propertyB".to_string(), property("form_a"));

    let config = Config {
        properties,
        fields,
        forms,
        ..Config::default()
    };

    let input = "\
- [ ] new colleague: John #propertyA #propertyB
  __Name:__ John
";
    let items = p(input);
    let issues = colliding_field_labels(&items, &config);

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
    let issues = colliding_field_labels(&items, &config);

    assert!(issues.is_empty());
}

#[test]
fn each_node_in_the_tree_is_checked_independently() {
    let input = "\
- [ ] parent
  - [ ] sub #propertyA #propertyB
    __Name:__ John
";
    let items = p(input);
    let config = config_with_colliding_forms();
    let issues = colliding_field_labels(&items, &config);

    assert_eq!(issues.len(), 1);
}
