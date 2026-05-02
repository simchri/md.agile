use crate::parser::parse;
use std::path::PathBuf;

#[test]
fn detects_done_parent_with_incomplete_children() {
    let input = "\
- [x] parent task is done
  - [ ] but this child is still todo
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::incomplete_parent(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, "E004");
    assert_eq!(issues[0].location.line, 1); // Parent task line
}

#[test]
fn allows_done_parent_with_all_done_children() {
    let input = "\
- [x] parent is done
  - [x] child is done
  - [x] another child done
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::incomplete_parent(&items);

    assert_eq!(issues.len(), 0);
}

#[test]
fn allows_done_parent_with_all_cancelled_children() {
    let input = "\
- [x] parent is done
  - [-] child was cancelled
  - [-] another cancelled
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::incomplete_parent(&items);

    assert_eq!(issues.len(), 0);
}

#[test]
fn allows_todo_parent_with_incomplete_children() {
    let input = "\
- [ ] parent is todo
  - [ ] child is todo
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::incomplete_parent(&items);

    assert_eq!(issues.len(), 0);
}

#[test]
fn detects_in_deeply_nested_tasks() {
    let input = "\
- [x] level 1 done
  - [x] level 2 done
    - [ ] level 3 todo (should trigger)
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::incomplete_parent(&items);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, "E004");
    assert_eq!(issues[0].location.line, 2); // Level 2 task with incomplete child
}

#[test]
fn ignores_cancelled_parents() {
    let input = "\
- [-] parent is cancelled
  - [ ] child is still todo
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::incomplete_parent(&items);

    assert_eq!(issues.len(), 0);
}

#[test]
fn allows_done_parent_with_optional_incomplete_children() {
    let input = "\
- [x] parent is done
  - [ ] #OPT optional child (incomplete is OK)
";
    let items = parse(input, PathBuf::from("test.agile.md"));
    let issues = super::incomplete_parent(&items);

    // Optional subtasks don't block parent completion
    assert_eq!(issues.len(), 0);
}
