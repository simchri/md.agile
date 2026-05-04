use super::*;
use crate::parser::Location;
use crate::rules::{ErrorCode, Issue};
use std::path::PathBuf;

fn dummy_issue(code: ErrorCode) -> Issue {
    Issue {
        location: Location {
            path: PathBuf::from("/no/such/file"),
            line: 1,
        },
        code,
        message: "msg".to_string(),
        column: 1,
        help: None,
        data: None,
    }
}

#[test]
fn format_issue_appends_fix_hint_when_quickfix_available() {
    let output = format_issue(&dummy_issue(ErrorCode::UppercaseX));
    assert!(output.contains("(fix avail.)"), "output was: {output}");
}

#[test]
fn format_issue_omits_fix_hint_when_no_quickfix() {
    let output = format_issue(&dummy_issue(ErrorCode::OrphanedSubtask));
    assert!(!output.contains("(fix avail.)"));
}

#[test]
fn create_pointer_simple() {
    let line = "- [ ] task";
    assert_eq!(create_pointer(line, 1), "^");
    assert_eq!(create_pointer(line, 3), "  ^");
}

#[test]
fn create_pointer_with_indent() {
    let line = "  - [ ] indented task";
    assert_eq!(create_pointer(line, 1), "^");
    assert_eq!(create_pointer(line, 3), "  ^");
    assert_eq!(create_pointer(line, 5), "    ^");
}
