use crate::parser::parse;
use std::path::PathBuf;

#[test]
fn detects_body_line_with_wrong_indentation() {
    let input = "\
- [ ] task title
  description line at correct indent
   description line with extra space (wrong)
  description line back to correct
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::wrong_body_indent(&items);

    // Should report the misaligned line
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::WrongBodyIndentation);
    assert_eq!(issues[0].location.line, 3); // Line with extra space
}

#[test]
fn task_with_correct_body_indentation_passes() {
    let input = "\
- [ ] task title
  line one
  line two
  line three
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::wrong_body_indent(&items);
    assert_eq!(issues.len(), 0);
}

#[test]
fn subtask_body_indentation_checked() {
    let input = "\
- [ ] parent task
  - [ ] subtask
    correct body line
     wrong body line
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::wrong_body_indent(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::WrongBodyIndentation);
}

#[test]
fn empty_body_has_no_issues() {
    let input = "\
- [ ] task with no body
- [ ] another task
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::wrong_body_indent(&items);
    assert_eq!(issues.len(), 0);
}
