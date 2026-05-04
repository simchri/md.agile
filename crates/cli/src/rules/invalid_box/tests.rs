use super::*;
use crate::parser::parse;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn accepts_todo_box() {
    let input = "\
- [ ] task
";
    assert!(invalid_box(&p(input)).is_empty());
}

#[test]
fn accepts_done_box() {
    let input = "\
- [x] task
";
    assert!(invalid_box(&p(input)).is_empty());
}

#[test]
fn accepts_cancelled_box() {
    let input = "\
- [-] task
";
    assert!(invalid_box(&p(input)).is_empty());
}

#[test]
fn accepts_valid_nested_boxes() {
    let input = "\
- [ ] parent
  - [x] done child
    - [-] cancelled grandchild
";
    assert!(invalid_box(&p(input)).is_empty());
}

#[test]
fn flags_task_with_empty_box() {
    let input = "\
- [] task
";
    let issues = invalid_box(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::BoxStyleInvalid);
    assert_eq!(issues[0].message, "Box style invalid");
}

#[test]
fn flags_task_with_other_symbol() {
    let input = "\
- [o] task
";
    let issues = invalid_box(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::BoxStyleInvalid);
    assert_eq!(issues[0].message, "Box style invalid");
}

#[test]
fn flags_subtask() {
    let input = "\
- [ ] task
  - [] task
";
    let issues = invalid_box(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::BoxStyleInvalid);
    assert_eq!(issues[0].message, "Box style invalid");
}

#[test]
fn flags_multiple_invalid_tasks() {
    let input = "\
- [] first
- [o] second
- [ ] valid
";
    let issues = invalid_box(&p(input));
    assert_eq!(issues.len(), 2);
    assert!(issues.iter().all(|i| i.code == crate::rules::ErrorCode::BoxStyleInvalid));
}

#[test]
fn flags_invalid_task_and_invalid_subtask() {
    let input = "\
- [] bad parent
  - [o] bad child
";
    let issues = invalid_box(&p(input));
    assert_eq!(issues.len(), 2);
    assert!(issues.iter().all(|i| i.code == crate::rules::ErrorCode::BoxStyleInvalid));
}

#[test]
fn flags_deeply_nested_subtask() {
    let input = "\
- [ ] top
  - [ ] level two
    - [ ] level three
      - [] level four invalid
";
    let issues = invalid_box(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::BoxStyleInvalid);
}
