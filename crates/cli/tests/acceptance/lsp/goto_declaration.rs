use super::helpers::start_project_session;

#[test]
fn lsp_goto_declaration_jumps_to_highest_priority_open_task() {
    // Arrange
    let (mut session, file_uri) = start_project_session("");
    let file_content = "\
- [x] done task
- [ ] highest priority open task
- [ ] another open task
";
    session.open_document(&file_uri, file_content);
    session.read_notification("textDocument/publishDiagnostics");

    // Act
    let response = session.goto_declaration(&file_uri, 2, 0, 0);

    // Assert
    assert!(
        !response["result"].is_null(),
        "expected a location result, got: {response}"
    );
    assert_eq!(
        response["result"]["uri"].as_str().unwrap(),
        file_uri,
        "GoTo Declaration should point to the tasks file"
    );
    assert_eq!(
        response["result"]["range"]["start"]["line"], 1,
        "GoTo Declaration should point to line 1 (highest priority open task)"
    );
}
