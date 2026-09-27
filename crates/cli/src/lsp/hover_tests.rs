use super::*;

#[test]
fn finds_opt_on_deeply_nested_subtask_without_matching_another_line() {
    let file_content = "\
- [ ] root
  - [ ] first child
    - [ ] second child
      - [ ] #OPT optional work
";
    let items = crate::parser::parse(file_content, PathBuf::from("hover.agile.md"));
    let FileItem::Task(task) = &items[0] else {
        panic!("expected a task");
    };
    let marker_line = file_content.lines().nth(3).unwrap();

    assert!(node_contains_marker(
        &task.markers,
        &task.children,
        task.location.line,
        task.indent,
        3,
        15,
        &SpecialMarkerKind::Opt,
        marker_line,
    ));
    assert!(!node_contains_marker(
        &task.markers,
        &task.children,
        task.location.line,
        task.indent,
        2,
        15,
        &SpecialMarkerKind::Opt,
        marker_line,
    ));
}
