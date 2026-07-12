use super::*;

#[test]
fn parse_range_accepts_simple_range() {
    assert_eq!(parse_range("2:4"), Ok((2, 4)));
}

#[test]
fn parse_range_accepts_single_element_range() {
    assert_eq!(parse_range("3:3"), Ok((3, 3)));
}

#[test]
fn parse_range_rejects_missing_colon() {
    assert!(parse_range("24").is_err());
}

#[test]
fn parse_range_rejects_non_numeric_bounds() {
    assert!(parse_range("a:4").is_err());
    assert!(parse_range("2:b").is_err());
}

#[test]
fn parse_range_rejects_zero_start() {
    assert!(parse_range("0:4").is_err());
}

#[test]
fn parse_range_rejects_zero_end() {
    assert!(parse_range("1:0").is_err());
}

#[test]
fn parse_range_rejects_start_greater_than_end() {
    assert!(parse_range("4:2").is_err());
}

#[test]
fn apply_range_selects_inclusive_slice() {
    let items = vec!["a", "b", "c", "d", "e"];
    assert_eq!(apply_range(items, (2, 4)), vec!["b", "c", "d"]);
}

#[test]
fn apply_range_clamps_end_beyond_length() {
    let items = vec!["a", "b", "c"];
    assert_eq!(apply_range(items, (2, 10)), vec!["b", "c"]);
}

#[test]
fn apply_range_returns_empty_when_start_beyond_length() {
    let items = vec!["a", "b", "c"];
    assert_eq!(apply_range(items, (5, 10)), Vec::<&str>::new());
}

#[test]
fn apply_range_single_element() {
    let items = vec!["a", "b", "c"];
    assert_eq!(apply_range(items, (2, 2)), vec!["b"]);
}
