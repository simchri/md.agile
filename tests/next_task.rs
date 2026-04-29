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
    let expected = Some("[ ] the next task\n  [x] done subtask\n  [ ] pending subtask\n".to_string());
    assert_eq!(next_task(input), expected);
}

#[test]
fn next_task_returns_none_when_all_done() {
    let input = "\
- [x] done task one
- [x] done task two
";
    let expected: Option<String> = None;
    assert_eq!(next_task(input), expected);
}

#[test]
fn next_task_skips_cancelled() {
    let input = "\
- [-] cancelled task
- [ ] actual next task
";
    let expected = Some("[ ] actual next task\n".to_string());
    assert_eq!(next_task(input), expected);
}
