use crate::parser::parse;
use crate::rules::ErrorCode;
use std::path::PathBuf;

#[test]
fn allows_ranked_siblings_with_distinct_ranks_in_any_order() {
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
fn detects_duplicate_rank_among_siblings() {
    let file_content = "\
- [ ] make app more responsive
  - [ ] 1. add performance UI test
  - [ ] 1. refactor signals
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let issues = super::invalid_order(&items);

    assert_eq!(issues.len(), 2, "both duplicate-rank siblings are flagged");
    assert!(
        issues
            .iter()
            .all(|i| i.code == ErrorCode::DuplicateOrderRank)
    );
    assert_eq!(issues[0].location.line, 2);
    assert_eq!(issues[1].location.line, 3);
}

#[test]
fn duplicate_rank_check_is_scoped_to_direct_siblings_only() {
    // Same rank "1." used by two subtasks nested under different parents —
    // not siblings of each other, so this is not a duplicate.
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
fn detects_ranked_task_done_while_lower_ranked_sibling_incomplete() {
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
}

#[test]
fn allows_ranked_tasks_completed_in_sequence() {
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
fn cancelled_lower_ranked_sibling_does_not_block_completion() {
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
fn unranked_siblings_never_block_or_get_flagged() {
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
fn checks_ranking_at_every_nesting_level_independently() {
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
