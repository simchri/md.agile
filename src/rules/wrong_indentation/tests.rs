use super::*;
use crate::parser::parse;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn flags_subtask_with_mismatched_indent() {
    // Subtask with 3 spaces instead of 2 (depth 1).
    let input = "\
- [ ] top
   - [ ] sub with 3 spaces instead of 2
";
    let issues = wrong_indentation(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, "E002");
    assert_eq!(issues[0].message, "Wrong Indentation");
}

#[test]
fn passes_correctly_indented() {
    let input = "\
- [ ] top
  - [ ] depth 1
    - [ ] depth 2
      - [ ] depth 3
";
    let issues = wrong_indentation(&p(input));
    assert!(issues.is_empty());
}
