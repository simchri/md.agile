//! Rendering pieces shared by the terminal chart and the HTML/SVG chart:
//! the plot legend, trend-line equations, latest-point stats, and ANSI
//! color helpers.

use super::chart_trends::ChartTrends;
use super::date_utils::date_from_unix_days;
use super::plot_data::TodoDonePlotPoint;
use super::trend::{DAYS_PER_WEEK, LinearTrend};

pub(super) fn ansi_rgb_sample(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{r};{g};{b}m....\x1b[0m")
}

/// Colors `text` itself (as opposed to [`ansi_rgb_sample`]'s fixed "...."
/// swatch), for labels that need to stay readable (e.g. trend equations).
pub(super) fn ansi_rgb_text(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m")
}

/// Legend for the plot's four data lines. In `ascii` mode, the fixed
/// symbols used by the ASCII chart (`o`/`@`/`O`/`0`/`Q`) are shown
/// alongside their colors, since not every ASCII-only terminal renders
/// ANSI color; in the default mode (Braille chart), color is the only
/// differentiator, matching the Braille chart's palette. When
/// `color` is false (`--no-color`), the color swatches are omitted
/// entirely — in `ascii` mode the symbols alone stay meaningful, but in
/// the default mode the legend degrades to a plain, uncolored list.
pub(super) fn render_plot_legend(ascii: bool, color: bool) -> String {
    let red = if color {
        ansi_rgb_sample(255, 0, 0)
    } else {
        String::new()
    };
    let green = if color {
        ansi_rgb_sample(0, 255, 0)
    } else {
        String::new()
    };
    let yellow = if color {
        ansi_rgb_sample(255, 255, 0)
    } else {
        String::new()
    };
    let cyan = if color {
        ansi_rgb_sample(0, 255, 255)
    } else {
        String::new()
    };
    let white = if color {
        ansi_rgb_sample(255, 255, 255)
    } else {
        String::new()
    };
    if ascii {
        format!(
            "{red} o total          {green} @ done\n{yellow} O total trend    {cyan} 0 done trend\n{white} Q today\n"
        )
    } else {
        format!(
            "{red} total          {green} done\n{yellow} total trend    {cyan} done trend\n{white} today\n"
        )
    }
}

/// Renders the fitted total/done trend lines as explicit `y = a + b*x`
/// equations, so the slope (creep/velocity) and intercept (cutoff: the
/// trend's weight at `x = 0`) that drive the chart and the ETA are visible,
/// not just implied by the drawn lines. The slope is shown in weight/week
/// — matching the unit `--velocity` reports — with `x` in weeks since
/// `anchor_x_d` (or a plain point index when no real dates are
/// available — see `LinearTrend::anchor_x_d`).
pub(super) fn render_trend_equations(
    total_trend: Option<LinearTrend>,
    done_trend: Option<LinearTrend>,
    color: bool,
) -> String {
    let anchor_x_d = total_trend.or(done_trend).and_then(|t| t.anchor_x_d);
    let x_desc = match anchor_x_d.and_then(date_from_unix_days) {
        Some(anchor) => format!("weeks since {anchor}"),
        None => "point index".to_string(),
    };
    let yellow = if color {
        ansi_rgb_text(255, 255, 0, "total")
    } else {
        "total".to_string()
    };
    let cyan = if color {
        ansi_rgb_text(0, 255, 255, "done")
    } else {
        "done".to_string()
    };
    format!(
        "Trend lines (x = {x_desc}):\n  {yellow} = {}\n  {cyan}  = {}\n",
        render_trend_equation(total_trend),
        render_trend_equation(done_trend),
    )
}

/// Convenience wrapper over [`render_trend_equations`] that pulls its
/// total/done trend straight from an already-computed [`ChartTrends`], so
/// callers that already have one (every chart renderer) don't each have to
/// unpack the same fields themselves.
pub(super) fn render_plot_trend_equations(trends: &ChartTrends, color: bool) -> String {
    render_trend_equations(trends.total_trend, trends.done_trend, color)
}

/// Renders a single trend line as `<intercept> + <slope>/week * x`, or
/// "unknown" when the trend couldn't be fit. The fitted slope is per day;
/// it's converted to weight/week here purely for display, to match
/// `--velocity`'s unit.
fn render_trend_equation(trend: Option<LinearTrend>) -> String {
    match trend {
        Some(t) => format!(
            "{:.2} + {:.2}/week * x",
            t.anchor_y_wt,
            t.slope_wtpd * DAYS_PER_WEEK
        ),
        None => "unknown".to_string(),
    }
}

pub(super) fn render_plot_stats(latest: &TodoDonePlotPoint) -> String {
    format!(
        "total:  {} tasks  (weight {:.2})\ndone:   {} tasks  (weight {:.2})\n",
        latest.total_count_t, latest.total_weight_wt, latest.done_count_t, latest.done_weight_wt,
    )
}

#[cfg(test)]
#[path = "chart_common_tests.rs"]
mod tests;
