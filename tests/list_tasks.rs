use mdagile::list_tasks;

#[test]
fn empty_input_produces_no_output() {
    assert_eq!(list_tasks(""), "".to_string());
}

#[test]
fn file_with_no_tasks_produces_no_output() {
    let input = "\
# Just a heading

Some notes, no tasks here.
";
    assert_eq!(list_tasks(input), "".to_string());
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
    assert_eq!(list_tasks(input), expected);
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
    assert_eq!(list_tasks(input), expected);
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
    assert_eq!(list_tasks(input), expected);
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
    assert_eq!(list_tasks(input), expected);
}
