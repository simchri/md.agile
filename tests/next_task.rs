use mdagile::cli::subcommands::task::next_task;
use mdagile::parser::{self, FileItem};
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parser::parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn empty_input_produces_no_output() {
    assert_eq!(next_task(&p("")), "".to_string());
}

#[test]
fn file_with_no_tasks_produces_no_output() {
    let input = "\
# Just a heading

Some notes, no tasks here.
";
    assert_eq!(next_task(&p(input)), "".to_string());
}

#[test]
fn all_cancelled_produces_no_output() {
    let input = "\
- [-] cancelled one
- [-] cancelled two
";
    assert_eq!(next_task(&p(input)), "".to_string());
}

#[test]
fn next_task_skips_done_and_returns_first_todo() {
    let input = "\
- [x] already done
- [ ] the next task
  - [x] done subtask
  - [ ] pending subtask
- [ ] another task
";
    let expected = "[ ] the next task\n  [x] done subtask\n  [ ] pending subtask\n".to_string();
    assert_eq!(next_task(&p(input)), expected);
}

#[test]
fn next_task_returns_empty_when_all_done() {
    let input = "\
- [x] done task one
- [x] done task two
";
    let expected = "".to_string();
    assert_eq!(next_task(&p(input)), expected);
}

#[test]
fn next_task_skips_cancelled() {
    let input = "\
- [-] cancelled task
- [ ] actual next task
";
    let expected = "[ ] actual next task\n".to_string();
    assert_eq!(next_task(&p(input)), expected);
}
