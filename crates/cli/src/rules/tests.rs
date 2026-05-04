use super::*;
use crate::parser::parse;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn wrong_indentation_vs_orphan_distinction_1() {
    // An Orphan task is:
    // - task that is separated by an empty line from previous elements
    // - AND has any indentation other than zero.
    //
    // Wrong indentation
    // - a sub task / task that is attached to a parent task (no empty line between other element)
    // - AND that has an indentation that does not match the subtask level

    let input = "\
- [ ] testing
  - [ ] subtask -> OK
  - [ ] another subtask -> OK
   - [ ] subtask WRONG INDENT
 - [ ] subtask WRONG INDENT

 - [ ] ORPHAN (also incorrect indent, but message should be orphan)

  - [ ] ORPHAN
";

    let mut issues = check_all(&p(input));
    issues.sort_by_key(|i| i.location.line);

    assert_eq!(issues[0].code, ErrorCode::WrongIndentation);
    assert_eq!(issues[1].code, ErrorCode::WrongIndentation);
    assert_eq!(issues[2].code, ErrorCode::OrphanedSubtask);
    assert_eq!(issues[3].code, ErrorCode::OrphanedSubtask);
}

#[test]
fn has_quickfix_for_each_code() {
    use ErrorCode::*;
    assert!(!OrphanedSubtask.has_quickfix());
    assert!(WrongIndentation.has_quickfix());
    assert!(WrongBodyIndentation.has_quickfix());
    assert!(!IncompleteParent.has_quickfix());
    assert!(MissingSpaceAfterBox.has_quickfix());
    assert!(BoxStyleInvalid.has_quickfix());
    assert!(UppercaseX.has_quickfix());
}

#[test]
fn missing_space_behind_box_vs_wrong_body_indent() {
    let input = "\
- [ ] ok, has space
- [ ]MISSING space
  - [ ] valid subtask
WRONG INDENT tasks description
";

    let mut issues = check_all(&p(input));
    issues.sort_by_key(|i| i.location.line);

    assert_eq!(issues[0].code, ErrorCode::MissingSpaceAfterBox);
    assert_eq!(issues[0].location.line, 2);

    assert_eq!(issues[1].code, ErrorCode::WrongBodyIndentation);
    assert_eq!(issues[1].location.line, 4);
}
