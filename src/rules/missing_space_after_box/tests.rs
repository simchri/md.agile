use crate::parser::parse;
use std::path::PathBuf;

#[test]
fn detects_missing_space_after_box() {
    let input = "\
- [ ]missing space (wrong)
- [ ] has space (correct)
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::missing_space_after_box(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::MissingSpaceAfterBox);
    assert_eq!(issues[0].location.line, 1); // First task has no space
}

#[test]
fn task_with_space_after_box_passes() {
    let input = "\
- [ ] task with proper space
- [x] done task with space
- [-] cancelled with space
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::missing_space_after_box(&items);

    assert_eq!(issues.len(), 0);
}

#[test]
fn detects_in_subtasks() {
    let input = "\
- [ ] parent
  - [ ]subtask without space
  - [ ] subtask with space
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::missing_space_after_box(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, crate::rules::ErrorCode::MissingSpaceAfterBox);
    assert_eq!(issues[0].location.line, 2); // Subtask without space
}

#[test]
fn detects_multiple_missing_spaces() {
    let input = "\
- [ ]no space 1
- [ ] space
- [x]no space 2
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::missing_space_after_box(&items);

    assert_eq!(issues.len(), 2);
    assert!(issues.iter().any(|i| i.location.line == 1));
    assert!(issues.iter().any(|i| i.location.line == 3));
}
