use mdagile::list_tasks;

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
