use crate::parser::parse;
use crate::rules::ErrorCode;
use std::path::PathBuf;

#[test]
fn allows_ordered_siblings_with_distinct_order_numbers_in_any_order() {
    let file_content = "\
- [ ] make app more responsive
  - [ ] 1. add performance UI test
  - [ ] 2. refactor signals
  - [ ] 4. document learnings
  - [ ] 3. run UI tests
  - [ ] 2 or more test users agree that performance is sufficient
  - [ ] discuss further steps
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues, vec![]);
}

#[test]
fn detects_duplicate_order_number_among_siblings() {
    let file_content = "\
- [ ] make app more responsive
  - [ ] 1. add performance UI test
  - [ ] 1. refactor signals
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(
        issues.len(),
        2,
        "both duplicate-order-number siblings are flagged"
    );
    assert!(
        issues
            .iter()
            .all(|i| i.code == ErrorCode::DuplicateOrderNumber)
    );
    assert_eq!(issues[0].location.line, 2);
    assert_eq!(issues[1].location.line, 3);
    // "  - [ ] 1. add performance UI test" — the "1" sits at column 9
    // (2 spaces indent + 6-char "- [ ] " prefix + 1), not column 1.
    assert_eq!(
        issues[0].column, 9,
        "column should point to the order number, not column 1"
    );
    assert_eq!(issues[1].column, 9);
}

#[test]
fn duplicate_order_number_column_points_past_the_opening_quote_for_property_required_subtasks() {
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"1. dev implementation\"
  - [ ] \"1. dev documentation\"
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues.len(), 2);
    // "  - [ ] \"1. dev implementation\"" — the "1" sits at column 10
    // (2 spaces indent + 6-char "- [ ] " prefix + 1 for the opening quote + 1).
    assert_eq!(issues[0].column, 10);
    assert_eq!(issues[1].column, 10);
}

#[test]
fn duplicate_order_number_check_is_scoped_to_direct_siblings_only() {
    // Same order number "1." used by two subtasks nested under different
    // parents — not siblings of each other, so this is not a duplicate.
    let file_content = "\
- [ ] parent A
  - [ ] 1. first step
- [ ] parent B
  - [ ] 1. first step
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues, vec![]);
}

#[test]
fn detects_ordered_task_done_while_lower_numbered_sibling_incomplete() {
    let file_content = "\
- [ ] make app more responsive
  - [ ] 1. add performance UI test
  - [x] 2. refactor signals
  - [ ] 4. document learnings
  - [ ] 3. run UI tests
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::OutOfOrderCompletion);
    assert_eq!(issues[0].location.line, 3); // "2. refactor signals"
    // "  - [x] 2. refactor signals" — the "2" sits at column 9, not column 1.
    assert_eq!(
        issues[0].column, 9,
        "column should point to the order number, not column 1"
    );
}

#[test]
fn allows_ordered_tasks_completed_in_sequence() {
    let file_content = "\
- [ ] make app more responsive
  - [x] 1. add performance UI test
  - [x] 2. refactor signals
  - [ ] 3. run UI tests
  - [ ] 4. document learnings
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues, vec![]);
}

#[test]
fn cancelled_lower_numbered_sibling_does_not_block_completion() {
    let file_content = "\
- [ ] make app more responsive
  - [-] 1. add performance UI test
  - [x] 2. refactor signals
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues, vec![]);
}

#[test]
fn unordered_siblings_never_block_or_get_flagged() {
    let file_content = "\
- [ ] make app more responsive
  - [ ] discuss further steps
  - [ ] 2 or more test users agree that performance is sufficient
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues, vec![]);
}

#[test]
fn checks_order_at_every_nesting_level_independently() {
    let file_content = "\
- [ ] top level task
  - [ ] 1. nested step
    - [ ] 1. deeply nested step
    - [x] 2. deeply nested step done out of order
  - [ ] 2. nested step
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::OutOfOrderCompletion);
    assert_eq!(issues[0].location.line, 4);
}

#[test]
fn detects_duplicate_order_number_among_property_required_subtasks() {
    // Order detection must also apply to property-required (quoted)
    // subtasks whose order prefix is baked into the quoted string, per
    // README.vision.md "Ordered Tasks via Properties".
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"1. dev implementation\"
  - [ ] \"1. dev documentation\"
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues.len(), 2);
    assert!(
        issues
            .iter()
            .all(|i| i.code == ErrorCode::DuplicateOrderNumber)
    );
}

#[test]
fn detects_property_required_subtask_completed_out_of_order() {
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"1. dev implementation\"
  - [x] \"2. dev documentation\"
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::OutOfOrderCompletion);
    assert_eq!(issues[0].location.line, 3); // "2. dev documentation"
}

#[test]
fn allows_property_required_subtasks_completed_in_sequence() {
    let file_content = "\
- [x] #feature: add basket
  - [x] \"1. dev implementation\"
  - [x] \"2. dev documentation\"
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues, vec![]);
}
