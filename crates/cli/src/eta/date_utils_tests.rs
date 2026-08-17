use super::*;

#[test]
fn date_from_unix_days_zero_is_the_unix_epoch() {
    assert_eq!(date_from_unix_days(0), NaiveDate::from_ymd_opt(1970, 1, 1));
}

#[test]
fn date_from_unix_days_round_trips_through_unix_days_from_date() {
    for days in [-30_000, -1, 0, 1, 19_723, 700_000] {
        let date = date_from_unix_days(days).expect("valid date");
        assert_eq!(unix_days_from_date(date), days);
    }
}

#[test]
fn unix_days_from_date_matches_a_known_calendar_date() {
    let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    assert_eq!(unix_days_from_date(date), 20_454);
}
