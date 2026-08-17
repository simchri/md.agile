//! ETA text formatting/rendering: turning a resolved [`EtaEstimate`] (and
//! "today") into the human-readable "ETA: ... / ETA date: ..." text block
//! shown after every chart. Pure string formatting — no trend-fitting (see
//! `trend.rs`) and no ETA date/time math (see `eta_math.rs`).

use super::date_utils::date_from_unix_days;
use super::eta_math::EtaEstimate;

/// Formats a day count as a human-friendly time span. Per README.vision.md:
/// days below a week, weeks below 8 weeks, years from 3 years and higher,
/// months otherwise.
fn format_days_as_span(days: i64) -> String {
    const DAYS_PER_WEEK: i64 = 7;
    const DAYS_PER_MONTH: f64 = 30.44;
    const DAYS_PER_YEAR: f64 = 365.25;
    const YEAR_THRESHOLD_DAYS: i64 = 3 * 365;

    if days < DAYS_PER_WEEK {
        return pluralize(days, "day");
    }
    if days < 8 * DAYS_PER_WEEK {
        return pluralize((days as f64 / DAYS_PER_WEEK as f64).round() as i64, "week");
    }
    if days < YEAR_THRESHOLD_DAYS {
        return pluralize((days as f64 / DAYS_PER_MONTH).round() as i64, "month");
    }
    pluralize((days as f64 / DAYS_PER_YEAR).round() as i64, "year")
}

fn pluralize(n: i64, unit: &str) -> String {
    if n == 1 {
        format!("1 {unit}")
    } else {
        format!("{n} {unit}s")
    }
}

/// Resolves an ETA (and "today") down to its human-readable span, or `None`
/// if either half is missing (meaning "unknown" to callers).
pub(super) fn eta_span(eta: Option<EtaEstimate>, today_unix_days: Option<i64>) -> Option<String> {
    let (eta, today) = (eta?, today_unix_days?);
    Some(format_days_as_span(eta.unix_days - today))
}

/// Renders the "ETA: ..." / "ETA date: ..." text block shown after the plot.
/// All string formatting (date formatting and the day-count-to-span
/// conversion) lives here; [`super::eta_math::compute_eta`] only ever deals
/// in dates.
pub(super) fn render_eta_text(eta: Option<EtaEstimate>, today_unix_days: Option<i64>) -> String {
    let Some(span) = eta_span(eta, today_unix_days) else {
        return format!("{:<10}unknown\n", "ETA:");
    };
    let date = date_from_unix_days(eta.unwrap().unix_days)
        .map(|d| d.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!("{:<10}{span}\n{:<10}{date}\n", "ETA:", "ETA date:")
}

#[cfg(test)]
#[path = "eta_text_tests.rs"]
mod tests;
