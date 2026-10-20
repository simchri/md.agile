use super::*;
use crate::config::{FieldConfig, FormConfig};

#[test]
fn active_branch_form_field_offers_allowed_values() {
    let config = Config::from_str(
        "\
[Field.answer]
label = \"Answer\"
values = [\"yes\", \"no\"]
help = \"Choose an answer\"
[Form.details]
fields = [\"answer\"]
[Properties.review]
[Properties.review.passed]
form = \"details\"
",
    )
    .unwrap();
    let file_content = "\
- [ ] #review:passed
  __Answer:__ ___
";

    let result = field_value_completions(file_content, at(1, 15), &config);

    assert_eq!(labels_and_edits(result).len(), 2);
}

#[test]
fn active_branch_form_field_resolves_its_help() {
    let config = Config::from_str(
        "\
[Field.answer]
label = \"Answer\"
help = \"Choose an answer\"
[Form.details]
fields = [\"answer\"]
[Properties.review]
[Properties.review.passed]
form = \"details\"
",
    )
    .unwrap();
    let file_content = "\
- [ ] #review:passed
  __Answer:__ ___
";

    let (_, field) = field_config_at_line(file_content, 1, &config).unwrap();

    assert_eq!(field.help.as_deref(), Some("Choose an answer"));
}

#[test]
fn pending_branch_does_not_resolve_inactive_form_fields() {
    let config = Config::from_str(
        "\
[Field.answer]
label = \"Answer\"
[Form.details]
fields = [\"answer\"]
[Properties.review]
[Properties.review.passed]
form = \"details\"
",
    )
    .unwrap();
    let file_content = "\
- [ ] #review:...
  __Answer:__ ___
";

    let field = field_config_at_line(file_content, 1, &config);

    assert!(field.is_none());
}

fn config_with_gender(values: Option<Vec<&str>>, multiline: bool) -> Config {
    let mut config = Config::default();
    config.fields.insert(
        "gender".to_string(),
        FieldConfig {
            name: "gender".to_string(),
            label: "Gender".to_string(),
            default: None,
            default_is_valid_entry: false,
            values: values.map(|v| v.into_iter().map(String::from).collect()),
            multiline,
            help: None,
        },
    );
    config.forms.insert(
        "personal".to_string(),
        FormConfig {
            name: "personal".to_string(),
            fields: vec!["gender".to_string()],
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

fn labels_and_edits(response: Option<CompletionResponse>) -> Vec<(String, String, u32, u32)> {
    let Some(CompletionResponse::Array(items)) = response else {
        return vec![];
    };
    items
        .into_iter()
        .map(|item| {
            let Some(CompletionTextEdit::Edit(edit)) = item.text_edit else {
                panic!("expected a text edit");
            };
            (
                item.label,
                edit.new_text,
                edit.range.start.character,
                edit.range.end.character,
            )
        })
        .collect()
}

const FILE_CONTENT: &str = "\
- [ ] new colleague #onboarding
  __Gender:__ ___
";

fn at(line: u32, character: u32) -> Position {
    Position { line, character }
}

#[test]
fn offers_all_values_replacing_the_placeholder() {
    // Arrange
    let config = config_with_gender(Some(vec!["male", "female"]), false);

    // Act
    let result = field_value_completions(FILE_CONTENT, at(1, 17), &config);

    // Assert
    assert_eq!(
        labels_and_edits(result),
        vec![
            ("male".to_string(), "male".to_string(), 14, 17),
            ("female".to_string(), "female".to_string(), 14, 17),
        ]
    );
}

#[test]
fn filters_by_typed_prefix_case_insensitively() {
    // Arrange
    let config = config_with_gender(Some(vec!["male", "female"]), false);
    let file_content = "\
- [ ] new colleague #onboarding
  __Gender:__ FE
";

    // Act
    let result = field_value_completions(file_content, at(1, 16), &config);

    // Assert
    assert_eq!(
        labels_and_edits(result),
        vec![("female".to_string(), "female".to_string(), 14, 16)]
    );
}

#[test]
fn offers_nothing_when_cursor_is_on_the_label() {
    let config = config_with_gender(Some(vec!["male"]), false);

    assert!(labels_and_edits(field_value_completions(FILE_CONTENT, at(1, 8), &config)).is_empty());
}

#[test]
fn offers_nothing_for_a_field_without_values() {
    let config = config_with_gender(None, false);

    assert!(labels_and_edits(field_value_completions(FILE_CONTENT, at(1, 17), &config)).is_empty());
}

#[test]
fn offers_nothing_for_a_multiline_field() {
    let config = config_with_gender(Some(vec!["male"]), true);

    assert!(labels_and_edits(field_value_completions(FILE_CONTENT, at(1, 17), &config)).is_empty());
}

#[test]
fn offers_nothing_when_task_has_no_property_with_a_form() {
    let config = config_with_gender(Some(vec!["male"]), false);
    let file_content = "\
- [ ] new colleague
  __Gender:__ ___
";

    assert!(labels_and_edits(field_value_completions(file_content, at(1, 17), &config)).is_empty());
}
