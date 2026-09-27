use super::*;

fn highest_priority_open_task_line(file_content: &str) -> Option<u32> {
    highest_priority_open_task(None, Path::new("tasks.agile.md"), Some(file_content), None)
        .map(|(_, line)| line)
}

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

#[test]
fn highest_priority_keeps_file_priority_before_current_buffer() {
    let project = tempfile::tempdir().unwrap();
    let first = project.path().join("01_tasks.agile.md");
    let second = project.path().join("02_tasks.agile.md");
    let file_content = "\
- [ ] earlier task
";
    std::fs::write(&first, file_content).unwrap();
    let file_content = "\
- [x] closed on disk
";
    std::fs::write(&second, file_content).unwrap();
    let file_content = "\
- [ ] reopened in editor
";

    assert_eq!(
        highest_priority_open_task(Some(project.path()), &second, Some(file_content), None),
        Some((first, 0)),
    );
}

#[test]
fn relative_navigation_uses_live_current_buffer_and_disk_for_other_files() {
    let project = tempfile::tempdir().unwrap();
    let first = project.path().join("01_tasks.agile.md");
    let second = project.path().join("02_tasks.agile.md");
    let file_content = "\
- [ ] first task
";
    std::fs::write(&first, file_content).unwrap();
    let file_content = "\
- [ ] task in next file
";
    std::fs::write(&second, file_content).unwrap();
    let file_content = "\
- [x] first task completed in editor
- [ ] second task added in editor
";

    assert_eq!(
        next_open_task_after(project.path(), &first, Some(file_content), 0),
        Some((first.clone(), 1)),
    );
    assert_eq!(
        next_open_task_after(project.path(), &first, Some(file_content), 1),
        Some((second, 0)),
    );
}
