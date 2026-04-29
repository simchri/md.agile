use mdagile::{list_tasks, next_task, read_task_files};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn setup_tasks_dir(root: &Path) {
    let tasks = root.join("tasks");
    fs::create_dir(&tasks).unwrap();
    fs::write(tasks.join("a_current.agile.md"), "- [ ] current task one\n- [ ] current task two\n").unwrap();
    fs::write(tasks.join("b_backlog.agile.md"), "- [ ] backlog task one\n- [ ] backlog task two\n").unwrap();
    fs::write(tasks.join("c_inbox.agile.md"),   "- [ ] inbox task one\n- [ ] inbox task two\n").unwrap();
}

#[test]
fn current_tasks_listed_before_backlog_before_inbox() {
    let dir = tempdir().unwrap();
    setup_tasks_dir(dir.path());

    let expected = "\
[ ] current task one
[ ] current task two
[ ] backlog task one
[ ] backlog task two
[ ] inbox task one
[ ] inbox task two
";
    assert_eq!(list_tasks(&read_task_files(dir.path())), expected);
}

#[test]
fn next_task_comes_from_current_not_backlog() {
    let dir = tempdir().unwrap();
    setup_tasks_dir(dir.path());

    let expected = "[ ] current task one\n".to_string();
    assert_eq!(next_task(&read_task_files(dir.path())), expected);
}

#[test]
fn next_task_falls_through_to_backlog_when_current_is_done() {
    let dir = tempdir().unwrap();
    let tasks = dir.path().join("tasks");
    fs::create_dir(&tasks).unwrap();
    fs::write(tasks.join("a_current.agile.md"), "- [x] current task one\n- [x] current task two\n").unwrap();
    fs::write(tasks.join("b_backlog.agile.md"), "- [ ] backlog task one\n").unwrap();
    fs::write(tasks.join("c_inbox.agile.md"),   "- [ ] inbox task one\n").unwrap();

    let expected = "[ ] backlog task one\n".to_string();
    assert_eq!(next_task(&read_task_files(dir.path())), expected);
}
