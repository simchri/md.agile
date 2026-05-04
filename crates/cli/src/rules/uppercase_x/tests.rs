use super::*;
use crate::parser::parse;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn flags_task_with_uppercase_x() {
    let input = "\
- [X] task
";
    let issues = uppercase_x(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::UppercaseX);
}

#[test]
fn flags_subtask_with_uppercase_x() {
    let input = "\
- [ ] task
  - [X] subtask
";
    let issues = uppercase_x(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::UppercaseX);
}

#[test]
fn accepts_lowercase_x() {
    let input = "\
- [x] task
";
    assert!(uppercase_x(&p(input)).is_empty());
}

#[test]
fn invalid_box_does_not_flag_uppercase_x() {
    // [X] is E007, not E006 — the two rules must not overlap
    use crate::rules::invalid_box;
    let input = "\
- [X] task
";
    assert!(invalid_box(&p(input)).is_empty());
}
