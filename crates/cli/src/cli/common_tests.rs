use super::*;
use crate::formatter::{BOLD, RESET};
use crate::parser::{self, FileItem};

fn parse_one_task(file_content: &str) -> parser::Task {
    let items = parser::parse(file_content, PathBuf::from("t.agile.md"));
    for item in items {
        if let FileItem::Task(task) = item {
            return task;
        }
    }
    panic!("no top-level task found in {file_content:?}");
}

#[test]
fn render_task_highlighting_next_leaf_bolds_the_only_todo_leaf() {
    let file_content = "\
- [ ] parent task
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, &mut out);
    assert_eq!(out, format!("{BOLD}[ ] parent task{RESET}\n"));
}

#[test]
fn render_task_highlighting_next_leaf_bolds_first_todo_leaf_in_document_order() {
    let file_content = "\
- [ ] parent task
  - [x] already done subtask
  - [ ] first todo leaf
  - [ ] second todo leaf
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] parent task\n  [x] already done subtask\n  {BOLD}[ ] first todo leaf{RESET}\n  [ ] second todo leaf\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_skips_non_leaf_todo_nodes() {
    // A `Todo` node with children is not itself a leaf, so it is never
    // bolded - only the actual leaf (a node with no children) is.
    let file_content = "\
- [ ] parent task
  - [ ] mid-level subtask with children
    - [ ] actual leaf
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] parent task\n  [ ] mid-level subtask with children\n    {BOLD}[ ] actual leaf{RESET}\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_bolds_nothing_when_no_todo_leaf_exists() {
    let file_content = "\
- [x] parent task
  - [x] done subtask
  - [-] cancelled subtask
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, &mut out);
    assert_eq!(
        out,
        "[x] parent task\n  [x] done subtask\n  [-] cancelled subtask\n"
    );
    assert!(!out.contains(BOLD));
}

#[test]
fn render_subtask_as_root_highlighting_next_leaf_bolds_within_its_own_subtree() {
    let file_content = "\
- [ ] parent task
  - [ ] addressed subtask
    - [x] already done grandchild
    - [ ] next leaf
";
    let task = parse_one_task(file_content);
    let addressed = &task.children[0];
    let mut out = String::new();
    render_subtask_as_root_highlighting_next_leaf(addressed, false, &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] addressed subtask\n  [x] already done grandchild\n  {BOLD}[ ] next leaf{RESET}\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_includes_body_when_requested() {
    let file_content = "\
- [ ] parent task
  some body text
  - [ ] leaf with a body
    leaf body line
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, true, &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] parent task\n  some body text\n  {BOLD}[ ] leaf with a body{RESET}\n    leaf body line\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_omits_body_by_default() {
    let file_content = "\
- [ ] parent task
  some body text
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, &mut out);
    assert_eq!(out, format!("{BOLD}[ ] parent task{RESET}\n"));
}
