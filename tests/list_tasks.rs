use mdagile::parser::{self, FileItem};
use mdagile::{active_task_blocks, list_tasks};
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parser::parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn active_excludes_done_and_cancelled() {
    let input = "\
- [x] done task
- [ ] active task
- [-] cancelled task
- [ ] another active task
";
    let expected = "\
[ ] active task
[ ] another active task
";
    assert_eq!(active_task_blocks(&p(input)).into_iter().collect::<String>(), expected);
}

#[test]
fn active_includes_todo_parent_with_done_subtasks() {
    let input = "\
- [x] done parent
  - [ ] subtask under done
- [ ] active parent
  - [x] done subtask
";
    let expected = "[ ] active parent\n  [x] done subtask\n";
    assert_eq!(active_task_blocks(&p(input)).into_iter().collect::<String>(), expected);
}

#[test]
fn active_empty_when_nothing_todo() {
    let input = "\
- [x] done one
- [-] cancelled one
";
    assert_eq!(active_task_blocks(&p(input)).into_iter().collect::<String>(), "".to_string());
}

#[test]
fn empty_input_produces_no_output() {
    assert_eq!(list_tasks(&p("")), "".to_string());
}

#[test]
fn file_with_no_tasks_produces_no_output() {
    let input = "\
# Just a heading

Some notes, no tasks here.
";
    assert_eq!(list_tasks(&p(input)), "".to_string());
}

#[test]
fn deeply_nested_tasks() {
    let input = "\
- [ ] top level
  - [ ] level two
    - [x] level three
";
    let expected = "\
[ ] top level
  [ ] level two
    [x] level three
";
    assert_eq!(list_tasks(&p(input)), expected);
}

#[test]
fn list_tasks_basic() {
    let input = "\
- [ ] implement feature X
  - [x] subtask one
  - [ ] subtask two

- [x] another task
- [-] a cancelled task
";
    let expected = "\
[ ] implement feature X
  [x] subtask one
  [ ] subtask two
[x] another task
[-] a cancelled task
";
    assert_eq!(list_tasks(&p(input)), expected);
}

#[test]
fn other_content_is_ignored() {
    let input = "\
# Sprint backlog

Some notes about the project.

- [ ] first task
- [x] second task

## Done

Nice work everyone.

- [-] third task (cancelled)
";
    let expected = "\
[ ] first task
[x] second task
[-] third task (cancelled)
";
    assert_eq!(list_tasks(&p(input)), expected);
}

#[test]
fn task_body_text_not_listed() {
    let input = "\
- [ ] implement feature X
  This is the task body. Some details about the task.
  More detail on another line.
  - [ ] a subtask

- [x] another task
  Body text here too.
";
    let expected = "\
[ ] implement feature X
  [ ] a subtask
[x] another task
";
    assert_eq!(list_tasks(&p(input)), expected);
}
