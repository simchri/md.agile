use super::*;

#[test]
fn parse_address_single_number() {
    assert_eq!(parse_address("2"), Some(vec![2]));
}

#[test]
fn parse_address_dotted() {
    assert_eq!(parse_address("1.3.2"), Some(vec![1, 3, 2]));
}

#[test]
fn parse_address_rejects_zero() {
    assert_eq!(parse_address("0"), None);
    assert_eq!(parse_address("1.0"), None);
}

#[test]
fn parse_address_rejects_empty_segment() {
    assert_eq!(parse_address(""), None);
    assert_eq!(parse_address("1."), None);
    assert_eq!(parse_address("1..2"), None);
    assert_eq!(parse_address(".1"), None);
}

#[test]
fn parse_address_rejects_non_numeric() {
    assert_eq!(parse_address("abc"), None);
    assert_eq!(parse_address("1.x"), None);
    assert_eq!(parse_address("-1"), None);
}

#[test]
fn set_status_done_replaces_empty_box() {
    let line = "- [ ] my task";
    assert_eq!(set_status_done(line, 0).as_deref(), Some("- [x] my task"));
}

#[test]
fn set_status_done_replaces_cancelled_box() {
    let line = "- [-] my task";
    assert_eq!(set_status_done(line, 0).as_deref(), Some("- [x] my task"));
}

#[test]
fn set_status_done_respects_indent() {
    let line = "  - [ ] nested subtask";
    assert_eq!(
        set_status_done(line, 2).as_deref(),
        Some("  - [x] nested subtask")
    );
}

#[test]
fn set_status_done_none_when_line_too_short() {
    assert_eq!(set_status_done("- [", 0), None);
}

#[test]
fn set_status_todo_replaces_done_box() {
    let line = "- [x] my task";
    assert_eq!(set_status_todo(line, 0).as_deref(), Some("- [ ] my task"));
}

#[test]
fn set_status_todo_respects_indent() {
    let line = "  - [x] nested subtask";
    assert_eq!(
        set_status_todo(line, 2).as_deref(),
        Some("  - [ ] nested subtask")
    );
}

#[test]
fn set_status_todo_none_when_line_too_short() {
    assert_eq!(set_status_todo("- [", 0), None);
}

fn write_temp_file(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let file = dir.path().join("tasks.agile.md");
    std::fs::write(&file, content).expect("write temp file");
    (dir, file)
}

#[test]
fn mark_node_done_marks_a_todo_task_and_returns_its_title() {
    let content = "\
- [ ] a simple task
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);
    let config = Config::default();

    let result = mark_node_done(&file, &items, 1, &config);
    assert_eq!(result.as_deref(), Ok("a simple task"));

    let written = std::fs::read_to_string(&file).unwrap();
    assert_eq!(written, "- [x] a simple task\n");
}

#[test]
fn mark_node_done_marks_a_nested_subtask() {
    let content = "\
- [ ] a task
  - [ ] a subtask
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);
    let config = Config::default();

    let result = mark_node_done(&file, &items, 2, &config);
    assert_eq!(result.as_deref(), Ok("a subtask"));

    let written = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        written,
        "\
- [ ] a task
  - [x] a subtask
"
    );
}

#[test]
fn mark_node_done_rejects_a_task_that_is_already_done() {
    let content = "\
- [x] already done
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);
    let config = Config::default();

    match mark_node_done(&file, &items, 1, &config) {
        Err(MarkDoneError::NotTodo(title)) => assert_eq!(title, "already done"),
        other => panic!("expected NotTodo, got {other:?}"),
    }
    // file must be untouched
    assert_eq!(std::fs::read_to_string(&file).unwrap(), content);
}

#[test]
fn mark_node_done_rejects_a_task_with_incomplete_required_children() {
    let content = "\
- [ ] a task
  - [ ] required subtask
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);
    let config = Config::default();

    match mark_node_done(&file, &items, 1, &config) {
        Err(MarkDoneError::RuleViolations(issues)) => assert!(!issues.is_empty()),
        other => panic!("expected RuleViolations, got {other:?}"),
    }
    // file must be untouched
    assert_eq!(std::fs::read_to_string(&file).unwrap(), content);
}

#[test]
fn mark_node_done_rejects_out_of_order_subtask_completion() {
    let content = "\
- [ ] a task
  - [ ] 1. first step
  - [ ] 2. second step
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);
    let config = Config::default();

    // Line 3 is "2. second step" — its lower-ordered sibling (line 2) is
    // still todo, so completing it out of order must be rejected.
    match mark_node_done(&file, &items, 3, &config) {
        Err(MarkDoneError::RuleViolations(issues)) => assert!(!issues.is_empty()),
        other => panic!("expected RuleViolations, got {other:?}"),
    }
    // file must be untouched
    assert_eq!(std::fs::read_to_string(&file).unwrap(), content);
}

#[test]
fn mark_node_done_allows_in_order_subtask_completion() {
    let content = "\
- [ ] a task
  - [ ] 1. first step
  - [ ] 2. second step
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);
    let config = Config::default();

    let result = mark_node_done(&file, &items, 2, &config);
    assert_eq!(result.as_deref(), Ok("first step"));
}

#[test]
fn mark_node_done_returns_not_found_when_no_node_starts_at_that_line() {
    let content = "\
- [ ] a task
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);
    let config = Config::default();

    assert!(matches!(
        mark_node_done(&file, &items, 99, &config),
        Err(MarkDoneError::NotFound)
    ));
}

#[test]
fn mark_node_undone_reverts_a_done_task_and_returns_its_title() {
    let content = "\
- [x] a finished task
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);

    let result = mark_node_undone(&file, &items, 1);
    assert_eq!(result.as_deref(), Ok("a finished task"));

    let written = std::fs::read_to_string(&file).unwrap();
    assert_eq!(written, "- [ ] a finished task\n");
}

#[test]
fn mark_node_undone_reverts_a_nested_done_subtask() {
    let content = "\
- [ ] a task
  - [x] a finished subtask
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);

    let result = mark_node_undone(&file, &items, 2);
    assert_eq!(result.as_deref(), Ok("a finished subtask"));

    let written = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        written,
        "\
- [ ] a task
  - [ ] a finished subtask
"
    );
}

#[test]
fn mark_node_undone_rejects_a_task_that_is_still_todo() {
    let content = "\
- [ ] still open
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);

    match mark_node_undone(&file, &items, 1) {
        Err(MarkUndoneError::NotDone(title)) => assert_eq!(title, "still open"),
        other => panic!("expected NotDone, got {other:?}"),
    }
    // file must be untouched
    assert_eq!(std::fs::read_to_string(&file).unwrap(), content);
}

#[test]
fn mark_node_undone_reverts_a_cancelled_task() {
    let content = "\
- [-] a cancelled task
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);

    match mark_node_undone(&file, &items, 1) {
        Err(MarkUndoneError::NotDone(title)) => assert_eq!(title, "a cancelled task"),
        other => panic!("expected NotDone, got {other:?}"),
    }
}

#[test]
fn mark_node_undone_returns_not_found_when_no_node_starts_at_that_line() {
    let content = "\
- [x] a task
";
    let (_dir, file) = write_temp_file(content);
    let items = parse_file(&file);

    assert!(matches!(
        mark_node_undone(&file, &items, 99),
        Err(MarkUndoneError::NotFound)
    ));
}
