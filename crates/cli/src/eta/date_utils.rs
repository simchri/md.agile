//! Date/time conversions shared across the ETA module: everything here
//! deals purely in unix days (days since the unix epoch) or `YYYY-MM-DD`
//! strings, with no plotting or reporting logic.

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns today's date as unix days (days since the unix epoch), or `None`
/// if the system clock is unavailable/invalid.
pub(super) fn today_unix_days() -> Option<i64> {
    unix_days_from_unix_seconds(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs() as i64),
    )
}

fn unix_days_from_unix_seconds(unix_seconds: Option<i64>) -> Option<i64> {
    unix_seconds.map(|s| s.div_euclid(86_400))
}

pub(super) fn parse_yyyy_mm_dd_to_unix_days(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let year = parts.next()?.parse::<i64>().ok()?;
    let month = parts.next()?.parse::<i64>().ok()?;
    let day = parts.next()?.parse::<i64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let yoe = adjusted_year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * month_prime + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub(super) fn format_yyyy_mm_dd_from_unix_days(unix_days: i64) -> String {
    let (year, month, day) = civil_from_days(unix_days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(unix_days: i64) -> (i64, i64, i64) {
    let z = unix_days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}
