use crate::parser::parse;
use crate::rules::{ErrorCode, IssueData};
use std::path::PathBuf;

fn p(input: &str) -> Vec<crate::parser::FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn detects_missing_space_after_label() {
    let input = "\
- [ ] new colleague: John
  __Email:__john.doe@gmail.com
";
    let items = p(input);
    let issues = super::missing_space_after_field_label(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::MissingSpaceAfterFieldLabel);
    assert_eq!(issues[0].location.line, 2);
    assert_eq!(
        issues[0].data,
        Some(IssueData::MissingSpaceAfterFieldLabel {
            label: "Email".to_string()
        })
    );
}

#[test]
fn well_formed_field_lines_pass() {
    let input = "\
- [ ] new colleague: John
  __Email:__ john.doe@gmail.com
  __Bio:__
  Some notes.
  ...
";
    let items = p(input);
    let issues = super::missing_space_after_field_label(&items);

    assert!(issues.is_empty());
}

#[test]
fn detects_in_subtasks() {
    let input = "\
- [ ] parent
  - [ ] sub
    __Email:__a@b.com
";
    let items = p(input);
    let issues = super::missing_space_after_field_label(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 3);
}

#[test]
fn detects_multiple_malformed_fields_on_the_same_node() {
    let input = "\
- [ ] new colleague: John
  __Email:__a@b.com
  __First Name:__John
";
    let items = p(input);
    let issues = super::missing_space_after_field_label(&items);

    assert_eq!(issues.len(), 2);
    assert!(issues.iter().any(|i| i.data
        == Some(IssueData::MissingSpaceAfterFieldLabel {
            label: "Email".to_string()
        })));
    assert!(issues.iter().any(|i| i.data
        == Some(IssueData::MissingSpaceAfterFieldLabel {
            label: "First Name".to_string()
        })));
}

#[test]
fn malformed_field_in_the_middle_of_an_otherwise_valid_multiline_capture_is_flagged() {
    let input = "\
- [ ] new colleague: John
  __Bio:__
  Some notes.
  __Email:__a@b.com
";
    let items = p(input);
    let issues = super::missing_space_after_field_label(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 4);
}
