//! HTML/SVG chart output: `agile when --plot --html`, rendering a
//! self-contained HTML file (inline SVG chart, no external dependencies/
//! network access) to disk.

use super::chart_common::{render_plot_stats, render_trend_equations};
use super::plot_data::{TodoDonePlot, TodoDonePlotPoint, compute_plot_y_range, x_axis_date_labels};
use super::trend::{LinearTrend, compute_eta, compute_plot_trends, render_eta_text};
use super::trend_geometry::trend_line_endpoints;
use std::path::Path;

/// Sanitizes a milestone name into the `[a-z0-9_]` slug used for the
/// `--html` output filename: lowercases, maps any run of characters
/// outside `[a-z0-9]` to a single underscore, and trims leading/trailing
/// underscores. Falls back to "milestone" if nothing alphanumeric remains.
fn sanitize_milestone_slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_was_underscore = false;
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_underscore = false;
        } else if !last_was_underscore {
            slug.push('_');
            last_was_underscore = true;
        }
    }
    let trimmed = slug.trim_matches('_');
    if trimmed.is_empty() {
        "milestone".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Renders `plot` as a self-contained HTML file (inline SVG chart, no
/// external dependencies/network access) and writes it to
/// "<milestone>-plot.html" in `root`, where `<milestone>` is
/// [`sanitize_milestone_slug`] applied to the milestone name. Returns the
/// path written to.
///
/// Shows the same information as the terminal plot (see
/// [`super::chart_terminal::render_todo_done_plot`]): the total/done data
/// lines, both fitted trend lines, today's marker, the legend, trend-line
/// equations, latest stats, and the ETA text.
pub fn write_todo_done_plot_html(
    root: &Path,
    plot: &TodoDonePlot,
    fit: bool,
) -> Result<std::path::PathBuf, String> {
    let html = render_todo_done_plot_html(plot, fit);
    let filename = format!(
        "{}-plot.html",
        sanitize_milestone_slug(&plot.milestone_name)
    );
    let path = root.join(filename);
    std::fs::write(&path, html).map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    Ok(path)
}

/// SVG canvas size (in pixels) for the `--html` chart.
const HTML_SVG_WIDTH: f64 = 900.0;
const HTML_SVG_HEIGHT: f64 = 420.0;
/// Margin around the plotted area, leaving room for axis labels.
const HTML_SVG_MARGIN_LEFT: f64 = 50.0;
const HTML_SVG_MARGIN_RIGHT: f64 = 20.0;
const HTML_SVG_MARGIN_TOP: f64 = 20.0;
const HTML_SVG_MARGIN_BOTTOM: f64 = 40.0;

fn render_todo_done_plot_html(plot: &TodoDonePlot, fit: bool) -> String {
    let today_unix_days = super::date_utils::today_unix_days();
    let trends = compute_plot_trends(plot, today_unix_days);
    let (ymin, ymax) = compute_plot_y_range(
        &trends.sampled,
        &trends.geometry,
        trends.total_trend,
        trends.done_trend,
        fit,
    );
    let eta = compute_eta(
        trends.total_trend,
        trends.done_trend,
        trends.geometry.anchor_unix_days,
        today_unix_days,
    );
    log::debug!(
        "render_todo_done_plot_html: {} sampled points, x range=[0, {:.3}], y range=[{ymin:.3}, {ymax:.3}]",
        trends.sampled.len(),
        trends.geometry.chart_x_max
    );

    let plot_w = HTML_SVG_WIDTH - HTML_SVG_MARGIN_LEFT - HTML_SVG_MARGIN_RIGHT;
    let plot_h = HTML_SVG_HEIGHT - HTML_SVG_MARGIN_TOP - HTML_SVG_MARGIN_BOTTOM;
    let xmax = trends.geometry.chart_x_max.max(1e-9);
    let yspan = (ymax - ymin).max(1e-9);
    let to_svg_x = |x: f64| HTML_SVG_MARGIN_LEFT + (x / xmax) * plot_w;
    let to_svg_y = |y: f64| HTML_SVG_MARGIN_TOP + plot_h - ((y - ymin) / yspan) * plot_h;

    let total_points_attr = svg_polyline_points(
        &trends.sampled,
        &trends.geometry.x_values,
        &to_svg_x,
        &to_svg_y,
        |p| p.total_weight_wt,
    );
    let done_points_attr = svg_polyline_points(
        &trends.sampled,
        &trends.geometry.x_values,
        &to_svg_x,
        &to_svg_y,
        |p| p.done_weight_wt,
    );

    let total_trend_line = trends
        .total_trend
        .map(|t| svg_trend_line_attrs(t, trends.geometry.trend_end_x, &to_svg_x, &to_svg_y));
    let done_trend_line = trends
        .done_trend
        .map(|t| svg_trend_line_attrs(t, trends.geometry.trend_end_x, &to_svg_x, &to_svg_y));

    let today_x_svg = to_svg_x(trends.geometry.today_x);
    let top_y_svg = to_svg_y(ymax);
    let bottom_y_svg = to_svg_y(ymin);

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg viewBox=\"0 0 {HTML_SVG_WIDTH} {HTML_SVG_HEIGHT}\" xmlns=\"http://www.w3.org/2000/svg\" font-family=\"monospace\" font-size=\"12\">\n"
    ));
    svg.push_str(&format!(
        "  <rect x=\"0\" y=\"0\" width=\"{HTML_SVG_WIDTH}\" height=\"{HTML_SVG_HEIGHT}\" fill=\"white\"/>\n"
    ));
    // Axes.
    svg.push_str(&format!(
        "  <line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"black\" stroke-width=\"1\"/>\n",
        HTML_SVG_MARGIN_LEFT, bottom_y_svg, HTML_SVG_WIDTH - HTML_SVG_MARGIN_RIGHT, bottom_y_svg
    ));
    svg.push_str(&format!(
        "  <line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"black\" stroke-width=\"1\"/>\n",
        HTML_SVG_MARGIN_LEFT, top_y_svg, HTML_SVG_MARGIN_LEFT, bottom_y_svg
    ));
    // Today marker (white/gray dashed vertical line).
    svg.push_str(&format!(
        "  <line x1=\"{today_x_svg:.2}\" y1=\"{top_y_svg:.2}\" x2=\"{today_x_svg:.2}\" y2=\"{bottom_y_svg:.2}\" stroke=\"#888\" stroke-width=\"1\" stroke-dasharray=\"4,3\"/>\n"
    ));
    // Trend lines (drawn under the raw data lines).
    if let Some((x1, y1, x2, y2)) = total_trend_line {
        svg.push_str(&format!(
            "  <line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" stroke=\"#e6b800\" stroke-width=\"2\"/>\n"
        ));
    }
    if let Some((x1, y1, x2, y2)) = done_trend_line {
        svg.push_str(&format!(
            "  <line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" stroke=\"#00b3b3\" stroke-width=\"2\"/>\n"
        ));
    }
    // Raw data lines.
    svg.push_str(&format!(
        "  <polyline points=\"{total_points_attr}\" fill=\"none\" stroke=\"red\" stroke-width=\"2\"/>\n"
    ));
    svg.push_str(&format!(
        "  <polyline points=\"{done_points_attr}\" fill=\"none\" stroke=\"green\" stroke-width=\"2\"/>\n"
    ));
    // Axis labels: min/max y value, and start/end date on the x axis.
    svg.push_str(&format!(
        "  <text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\">{:.0}</text>\n",
        HTML_SVG_MARGIN_LEFT - 5.0,
        bottom_y_svg + 4.0,
        ymin
    ));
    svg.push_str(&format!(
        "  <text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\">{:.0}</text>\n",
        HTML_SVG_MARGIN_LEFT - 5.0,
        top_y_svg + 4.0,
        ymax
    ));
    if let Some((start_label, end_label)) = x_axis_date_labels(&trends.sampled, &trends.geometry) {
        svg.push_str(&format!(
            "  <text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\">{start_label}</text>\n",
            HTML_SVG_MARGIN_LEFT,
            bottom_y_svg + 18.0
        ));
        svg.push_str(&format!(
            "  <text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\">{end_label}</text>\n",
            HTML_SVG_WIDTH - HTML_SVG_MARGIN_RIGHT,
            bottom_y_svg + 18.0
        ));
    }
    svg.push_str("</svg>");

    let legend = format!(
        "<ul class=\"legend\">\n\
         \x20 <li><span class=\"swatch\" style=\"background:red\"></span>total</li>\n\
         \x20 <li><span class=\"swatch\" style=\"background:green\"></span>done</li>\n\
         \x20 <li><span class=\"swatch\" style=\"background:#e6b800\"></span>total trend</li>\n\
         \x20 <li><span class=\"swatch\" style=\"background:#00b3b3\"></span>done trend</li>\n\
         \x20 <li><span class=\"swatch\" style=\"background:#888\"></span>today</li>\n\
         </ul>"
    );
    let trend_equations = render_trend_equations(
        trends.total_trend,
        trends.done_trend,
        trends.geometry.anchor_unix_days,
        false,
    );
    let stats = plot
        .points
        .last()
        .map(render_plot_stats)
        .unwrap_or_default();
    let eta_text = render_eta_text(eta, today_unix_days);

    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         \x20 <meta charset=\"utf-8\">\n\
         \x20 <title>{title}</title>\n\
         \x20 <style>\n\
         \x20   body {{ font-family: monospace; margin: 2em; }}\n\
         \x20   pre {{ white-space: pre-wrap; }}\n\
         \x20   .legend {{ list-style: none; padding: 0; display: flex; gap: 1.5em; }}\n\
         \x20   .legend li {{ display: flex; align-items: center; gap: 0.4em; }}\n\
         \x20   .swatch {{ display: inline-block; width: 1em; height: 1em; border-radius: 50%; }}\n\
         \x20 </style>\n\
         </head>\n\
         <body>\n\
         \x20 <h1>Milestone: {title}</h1>\n\
         \x20 {svg}\n\
         \x20 {legend}\n\
         \x20 <pre>{trend_equations}</pre>\n\
         \x20 <pre>{stats}</pre>\n\
         \x20 <pre>{eta_text}</pre>\n\
         </body>\n\
         </html>\n",
        title = html_escape(&plot.milestone_name),
    )
}

fn svg_polyline_points(
    points: &[TodoDonePlotPoint],
    x_values: &[f64],
    to_svg_x: &impl Fn(f64) -> f64,
    to_svg_y: &impl Fn(f64) -> f64,
    y_of: impl Fn(&TodoDonePlotPoint) -> f64,
) -> String {
    points
        .iter()
        .zip(x_values.iter())
        .map(|(p, x)| format!("{:.2},{:.2}", to_svg_x(*x), to_svg_y(y_of(p))))
        .collect::<Vec<_>>()
        .join(" ")
}

fn svg_trend_line_attrs(
    trend: LinearTrend,
    trend_end_x: f64,
    to_svg_x: &impl Fn(f64) -> f64,
    to_svg_y: &impl Fn(f64) -> f64,
) -> (f64, f64, f64, f64) {
    let e = trend_line_endpoints(trend, trend_end_x);
    (
        to_svg_x(e.x0),
        to_svg_y(e.y0),
        to_svg_x(e.x1),
        to_svg_y(e.y1),
    )
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
#[path = "chart_html_tests.rs"]
mod tests;
