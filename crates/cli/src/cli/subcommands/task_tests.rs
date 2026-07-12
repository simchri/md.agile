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
