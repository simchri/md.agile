use mdagile::cli::common::{find_task_files, parse_files};
use mdagile::cli::subcommands::list::list_tasks;
use mdagile::cli::subcommands::task::next_task;
use std::fs;
use tempfile::tempdir;

#[test]
fn current_tasks_listed_before_backlog_before_inbox() {
    let dir = tempdir().unwrap();
    let tasks = dir.path().join("tasks");
    fs::create_dir(&tasks).unwrap();

    let current = "\
- [ ] current task one
- [ ] current task two
";
    let backlog = "\
- [ ] backlog task one
- [ ] backlog task two
";
    let inbox = "\
- [ ] inbox task one
- [ ] inbox task two
";
    fs::write(tasks.join("a_current.agile.md"), current).unwrap();
    fs::write(tasks.join("b_backlog.agile.md"), backlog).unwrap();
    fs::write(tasks.join("c_inbox.agile.md"), inbox).unwrap();

    let expected = "\
[ ] current task one
[ ] current task two
[ ] backlog task one
[ ] backlog task two
[ ] inbox task one
[ ] inbox task two
";
    let items = parse_files(&find_task_files(dir.path()));
    assert_eq!(list_tasks(&items), expected);
}

#[test]
fn next_task_comes_from_current_not_backlog() {
    let dir = tempdir().unwrap();
    let tasks = dir.path().join("tasks");
    fs::create_dir(&tasks).unwrap();

    let current = "\
- [ ] current task one
- [ ] current task two
";
    let backlog = "\
- [ ] backlog task one
- [ ] backlog task two
";
    let inbox = "\
- [ ] inbox task one
- [ ] inbox task two
";
    fs::write(tasks.join("a_current.agile.md"), current).unwrap();
    fs::write(tasks.join("b_backlog.agile.md"), backlog).unwrap();
    fs::write(tasks.join("c_inbox.agile.md"), inbox).unwrap();

    let expected = "[ ] current task one\n".to_string();
    let items = parse_files(&find_task_files(dir.path()));
    assert_eq!(next_task(&items), expected);
}

#[test]
fn next_task_falls_through_to_backlog_when_current_is_done() {
    let dir = tempdir().unwrap();
    let tasks = dir.path().join("tasks");
    fs::create_dir(&tasks).unwrap();

    let current = "\
- [x] current task one
- [x] current task two
";
    let backlog = "- [ ] backlog task one\n";
    let inbox   = "- [ ] inbox task one\n";
    fs::write(tasks.join("a_current.agile.md"), current).unwrap();
    fs::write(tasks.join("b_backlog.agile.md"), backlog).unwrap();
    fs::write(tasks.join("c_inbox.agile.md"), inbox).unwrap();

    let expected = "[ ] backlog task one\n".to_string();
    let items = parse_files(&find_task_files(dir.path()));
    assert_eq!(next_task(&items), expected);
}

#[test]
fn identical_filenames_ordered_by_directory_path() {
    // Two files both named 001.agile.md; priority must come from directory prefix
    let dir = tempdir().unwrap();
    let tasks = dir.path().join("tasks");
    let current = tasks.join("50_current");
    let backlog = tasks.join("60_backlog");
    fs::create_dir_all(&current).unwrap();
    fs::create_dir_all(&backlog).unwrap();

    fs::write(current.join("001.agile.md"), "- [ ] current task\n").unwrap();
    fs::write(backlog.join("001.agile.md"), "- [ ] backlog task\n").unwrap();

    let paths = find_task_files(dir.path());
    assert!(
        paths[0].ends_with("50_current/001.agile.md"),
        "50_current should come first, got {:?}",
        paths
    );

    let expected = "[ ] current task\n[ ] backlog task\n";
    let items = parse_files(&find_task_files(dir.path()));
    assert_eq!(list_tasks(&items), expected);
}
