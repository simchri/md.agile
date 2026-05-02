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

    assert_eq!(issues[0].code, "E002");
    assert_eq!(issues[1].code, "E002");
    assert_eq!(issues[2].code, "E001");
    assert_eq!(issues[3].code, "E001");
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

    assert_eq!(issues[0].code, "E005");
    assert_eq!(issues[0].location.line, 2);

    assert_eq!(issues[1].code, "E003");
    assert_eq!(issues[1].location.line, 4);
}
