use super::*;
use crate::parser::parse;
use crate::rules::IssueData;
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
    assert_eq!(issues[0].code, crate::rules::ErrorCode::WrongIndentation);
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

#[test]
fn issue_data_carries_expected_indent_for_subtask() {
    // Subtask at depth 1 should snap to 2 spaces.
    let input = "\
- [ ] top
   - [ ] sub with 3 spaces
";
    let issues = wrong_indentation(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::WrongIndent { expected_indent: 2 }),
    );
}

#[test]
fn issue_data_carries_expected_indent_for_attached_top_level_task() {
    // 1-space indent on a `- [ ]` line not preceded by a blank line: the
    // parser produces a top-level Task with indent>0 (E002, not orphan).
    // The autocorrect should snap to 2 spaces (one subtask level deeper than
    // the previous top-level task).
    let input = "\
- [ ] top
 - [ ] attached but mis-indented
";
    let issues = wrong_indentation(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::WrongIndent { expected_indent: 2 }),
    );
}

#[test]
fn issue_data_for_deeper_subtask_uses_depth_times_two() {
    // depth-2 subtask with 5 spaces instead of 4.
    let input = "\
- [ ] top
  - [ ] depth 1
     - [ ] depth 2 with 5 spaces
";
    let issues = wrong_indentation(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].data,
        Some(IssueData::WrongIndent { expected_indent: 4 }),
    );
}
