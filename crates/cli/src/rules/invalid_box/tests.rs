use super::*;
use crate::parser::parse;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
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
