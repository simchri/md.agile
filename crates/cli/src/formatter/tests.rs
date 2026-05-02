use super::*;

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
