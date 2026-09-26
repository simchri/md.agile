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

#[test]
fn considers_subtasks_jumping_into_open_subtask_not_parent_line() {
    let doc = "\
- [x] done task
- [ ] parent with subtasks
  - [ ] first subtask
  - [ ] second subtask
";
    // The parent (line 1) still has open descendant work, so the actionable
    // task is its first open subtask (line 2), not the parent itself.
    assert_eq!(highest_priority_open_task_line(doc), Some(2));
}

#[test]
fn considers_subtasks_falling_back_to_parent_once_all_are_resolved() {
    let doc = "\
- [ ] parent with resolved subtasks
  - [x] first subtask
  - [-] second subtask
";
    // Every descendant is done/cancelled, so there's nothing left to
    // delegate to — the parent itself (line 0) is the actionable unit.
    assert_eq!(highest_priority_open_task_line(doc), Some(0));
}
