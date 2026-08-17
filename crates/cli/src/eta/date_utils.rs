//! Date/time conversions shared across the ETA module: everything here
//! deals purely in [`chrono::NaiveDate`] calendar dates, or the unix-days
//! representation trend fitting needs for arithmetic, with no plotting or
//! reporting logic.

use chrono::{Datelike, Days, NaiveDate};
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns today's calendar date, or `None` if the system clock is
/// unavailable/invalid.
pub(super) fn today_date() -> Option<NaiveDate> {
    let unix_seconds = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    date_from_unix_days(unix_seconds.div_euclid(86_400))
}

/// Converts a unix-days offset (days since the unix epoch) to a calendar
/// date, or `None` on overflow.
pub(super) fn date_from_unix_days(unix_days: i64) -> Option<NaiveDate> {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1)?;
    if unix_days >= 0 {
        epoch.checked_add_days(Days::new(unix_days as u64))
    } else {
        epoch.checked_sub_days(Days::new((-unix_days) as u64))
    }
}

/// Converts a calendar date to its unix-days offset (days since the unix
/// epoch), for the arithmetic trend fitting/ETA computation need.
pub(super) fn unix_days_from_date(date: NaiveDate) -> i64 {
    date.num_days_from_ce() as i64 - EPOCH_NUM_DAYS_FROM_CE
}

/// [`NaiveDate::num_days_from_ce`] for the unix epoch (1970-01-01), used to
/// convert its day-count-from-CE into a day-count-from-unix-epoch.
const EPOCH_NUM_DAYS_FROM_CE: i64 = 719_163;

/// Convenience wrapper combining [`today_date`] and [`unix_days_from_date`]
/// for callers that only need today's unix-days offset (trend/ETA math),
/// not the calendar date itself.
pub(super) fn today_unix_days() -> Option<i64> {
    today_date().map(unix_days_from_date)
}

#[cfg(test)]
#[path = "date_utils_tests.rs"]
mod tests;
