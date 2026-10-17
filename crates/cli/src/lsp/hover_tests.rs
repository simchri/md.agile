use super::*;

#[test]
fn finds_opt_on_deeply_nested_subtask_without_matching_another_line() {
    let file_content = "\
- [ ] root
  - [ ] first child
    - [ ] second child
      - [ ] #OPT optional work
";
    let items = crate::parser::parse(file_content, PathBuf::from("hover.agile.md"));
    let FileItem::Task(task) = &items[0] else {
        panic!("expected a task");
    };
    let marker_line = file_content.lines().nth(3).unwrap();

    assert!(node_contains_marker(
        &task.markers,
        &task.children,
        task.location.line,
        task.indent,
        3,
        15,
        &SpecialMarkerKind::Opt,
        marker_line,
    ));
    assert!(!node_contains_marker(
        &task.markers,
        &task.children,
        task.location.line,
        task.indent,
        2,
        15,
        &SpecialMarkerKind::Opt,
        marker_line,
    ));
}

fn config_with_field(help: Option<&str>) -> Config {
    use crate::config::{FieldConfig, FormConfig};
    let mut config = Config::default();
    config.fields.insert(
        "name".to_string(),
        FieldConfig {
            name: "name".to_string(),
            label: "Name".to_string(),
            default: None,
            default_is_valid_entry: false,
            validation_regex: None,
            values: None,
            multiline: false,
            help: help.map(String::from),
        },
    );
    config.forms.insert(
        "personal".to_string(),
        FormConfig {
            name: "personal".to_string(),
            fields: vec!["name".to_string()],
            validation: None,
        },
    );
    config.properties.insert(
        "onboarding".to_string(),
        PropertyConfig {
            name: "onboarding".to_string(),
            brief: None,
            description: None,
            subtasks: vec![],
            subtasks_allow_cancel: vec![],
            neighbortasks: vec![],
            branches: Default::default(),
            form: Some("personal".to_string()),
        },
    );
    config
}

fn hover_text(hover: Option<Hover>) -> Option<String> {
    match hover?.contents {
        HoverContents::Markup(markup) => Some(markup.value),
        _ => None,
    }
}

const FIELD_FILE_CONTENT: &str = "\
- [ ] new colleague #onboarding
  __Name:__ ___
";

#[test]
fn field_hover_on_label_shows_configured_help() {
    let config = config_with_field(Some("Use the person's legal name"));

    let hover = field_hover(FIELD_FILE_CONTENT, 1, 5, &config);

    assert_eq!(
        hover_text(hover),
        Some("**Name**\n\nUse the person's legal name".to_string())
    );
}

#[test]
fn field_hover_is_absent_on_the_value() {
    let config = config_with_field(Some("help"));

    assert_eq!(
        hover_text(field_hover(FIELD_FILE_CONTENT, 1, 13, &config)),
        None
    );
}

#[test]
fn field_hover_is_absent_without_configured_help() {
    let config = config_with_field(None);

    assert_eq!(
        hover_text(field_hover(FIELD_FILE_CONTENT, 1, 5, &config)),
        None
    );
}

#[test]
fn field_hover_is_absent_when_task_has_no_property_with_a_form() {
    let config = config_with_field(Some("help"));
    let file_content = "\
- [ ] new colleague
  __Name:__ ___
";

    assert_eq!(hover_text(field_hover(file_content, 1, 5, &config)), None);
}

#[test]
fn field_hover_works_on_a_multiline_field_label() {
    let config = config_with_field(Some("help"));
    let file_content = "\
- [ ] new colleague #onboarding
  __Name:__
  ___
  ...
";

    assert_eq!(
        hover_text(field_hover(file_content, 1, 5, &config)),
        Some("**Name**\n\nhelp".to_string())
    );
}
