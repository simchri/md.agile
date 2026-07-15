use super::*;

#[test]
fn unix_to_yyyy_mm_dd_formats_epoch() {
    assert_eq!(unix_to_yyyy_mm_dd(0), "1970-01-01");
}

#[test]
fn unix_to_yyyy_mm_dd_formats_known_day() {
    // 2026-07-11 00:00:00 UTC
    assert_eq!(unix_to_yyyy_mm_dd(1_783_728_000), "2026-07-11");
}
