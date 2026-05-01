use super::*;
use crate::parser::parse;
use std::path::PathBuf;

#[test]
fn run_returns_no_issues_for_clean_input() {
    let input = "\
- [ ] top
  - [ ] sub
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    assert!(run(&items).is_empty());
}

#[test]
fn run_aggregates_rule_issues() {
    let input = "\
- [ ] top

  - [ ] orphan
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = run(&items);
    assert_eq!(issues.len(), 1);
}
