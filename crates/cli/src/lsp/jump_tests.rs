use super::*;

#[test]
fn finds_first_todo_task_in_text() {
    let doc = "\
- [x] done task
- [ ] first open task
- [ ] second open task
";
    assert_eq!(highest_priority_open_task_line(doc), Some(1));
}

#[test]
fn returns_none_when_no_todo_tasks() {
    let doc = "\
- [x] done task
- [-] cancelled task
";
    assert_eq!(highest_priority_open_task_line(doc), None);
}
