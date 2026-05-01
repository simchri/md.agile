use super::*;
use crate::parser::parse;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn flags_task_with_indent_after_blank_line() {
    let input = "\
- [ ] real top level

  - [ ] orphan indented
";
    let issues = orphaned_subtask(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 3);
    assert_eq!(issues[0].code, "E001");
    assert!(issues[0].message.contains("Orphaned"));
}

#[test]
fn passes_clean_file() {
    let input = "\
- [ ] top
  - [ ] proper sub
- [x] another top
";
    let issues = orphaned_subtask(&p(input));
    assert!(issues.is_empty());
}

#[test]
fn flags_multiple() {
    let input = "\
- [ ] top one

  - [ ] orphan a

- [ ] top two

    - [ ] orphan b
";
    let issues = orphaned_subtask(&p(input));
    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].location.line, 3);
    assert_eq!(issues[1].location.line, 7);
}
