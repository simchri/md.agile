use super::*;
use crate::parser::parse;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn flags_task_with_no_title_text() {
    let file_content = "\
- [ ] 
";
    let issues = empty_title(&p(file_content));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::EmptyTitle);
}

#[test]
fn flags_subtask_with_no_title_text() {
    let file_content = "\
- [ ] task
  - [ ] 
";
    let issues = empty_title(&p(file_content));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::EmptyTitle);
    assert_eq!(issues[0].column, 9); // indent(2) + "- [ ] ".len(6) + 1
}

#[test]
fn task_consisting_only_of_a_marker_is_not_flagged() {
    // Since markers now stay inline in the title, a marker-only line (e.g.
    // `#urgent`) is no longer considered empty — the marker text itself
    // counts as title text.
    let file_content = "\
- [ ] #urgent
";
    let issues = empty_title(&p(file_content));
    assert_eq!(issues.len(), 0);
}

#[test]
fn accepts_task_with_title_text() {
    let file_content = "\
- [ ] a real task
";
    assert!(empty_title(&p(file_content)).is_empty());
}

#[test]
fn accepts_task_with_marker_and_title_text() {
    let file_content = "\
- [ ] #urgent fix the bug
";
    assert!(empty_title(&p(file_content)).is_empty());
}
