use mdagile::next_task;

#[test]
fn next_task_skips_done_and_returns_first_todo() {
    let input = "\
- [x] already done
- [ ] the next task
  - [x] done subtask
  - [ ] pending subtask
- [ ] another task
";
    assert_eq!(
        next_task(input),
        Some("[ ] the next task\n  [x] done subtask\n  [ ] pending subtask\n".to_string())
    );
}

#[test]
fn next_task_returns_none_when_all_done() {
    let input = "\
- [x] done task one
- [x] done task two
";
    assert_eq!(next_task(input), None);
}

#[test]
fn next_task_skips_cancelled() {
    let input = "\
- [-] cancelled task
- [ ] actual next task
";
    assert_eq!(next_task(input), Some("[ ] actual next task\n".to_string()));
}
