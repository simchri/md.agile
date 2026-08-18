//! The default terminal chart backend: a Braille sub-pixel chart rendered
//! via `textplots`, giving higher effective resolution than one glyph per
//! terminal cell (see [`super::chart_terminal_ascii`] for the `--ascii`
//! fallback). `textplots`'s `Shape::Lines` draws straight canvas lines
//! directly between whichever points it's given (see its `figures()`,
//! which just walks the point pairs and calls `canvas.line`/
//! `canvas.line_colored`), so unlike the ASCII backend's fixed-width
//! character grid, this backend needs no downsampling: the caller
//! ([`super::render_todo_done_plot`]) feeds it the milestone's full,
//! unsampled point history.

use super::super::chart_trends::ChartTrends;
use super::super::plot_data::{TodoDonePlotPoint, x_axis_date_labels};
use super::super::trend_geometry::trend_line_endpoints_f32;
use super::{CHART_CHAR_HEIGHT, CHART_CHAR_WIDTH};
use rgb::RGB8;
use textplots::{Chart, ColorPlot, LabelBuilder, LabelFormat, Plot, Shape};

/// Braille sub-pixel packing factor used by the `textplots` backend: each
/// terminal cell holds a 2 (columns) x 4 (rows) grid of sub-pixels.
const BRAILLE_SUBPIXELS_X: usize = 2;
const BRAILLE_SUBPIXELS_Y: usize = 4;

const CHART_PIXEL_WIDTH: u32 = (CHART_CHAR_WIDTH * BRAILLE_SUBPIXELS_X) as u32;
const CHART_PIXEL_HEIGHT: u32 = (CHART_CHAR_HEIGHT * BRAILLE_SUBPIXELS_Y) as u32;

/// Builds one series as `(x, y)` `f32` pairs ready for `textplots`, pairing
/// each point with its already-computed x value. Shared by the total/done
/// data series (which otherwise differ only in which weight field they
/// read).
fn series_f32(
    points: &[TodoDonePlotPoint],
    x_values: &[f64],
    value_of: impl Fn(&TodoDonePlotPoint) -> f64,
) -> Vec<(f32, f32)> {
    points
        .iter()
        .zip(x_values.iter())
        .map(|(p, x)| (*x as f32, value_of(p) as f32))
        .collect()
}

pub(super) fn render_textplots_chart(trends: &ChartTrends, fit: bool, color: bool) -> String {
    let points = &trends.sampled;
    let geometry = &trends.geometry;
    let total_series = series_f32(points, &geometry.x_values, |p| p.total_weight_wt);
    let done_series = series_f32(points, &geometry.x_values, |p| p.done_weight_wt);
    let total_trend_series = trends
        .total_trend
        .map(|t| trend_line_endpoints_f32(t, geometry.trend_end_x).to_vec())
        .unwrap_or_default();
    let done_trend_series = trends
        .done_trend
        .map(|t| trend_line_endpoints_f32(t, geometry.trend_end_x).to_vec())
        .unwrap_or_default();
    let xmax = geometry.chart_x_max as f32;
    let (ymin, ymax) = trends.y_range(fit);
    let (ymin, ymax) = (ymin as f32, ymax as f32);
    let today_series = vec![
        (geometry.today_x as f32, ymin),
        (geometry.today_x as f32, ymax),
    ];
    log::debug!(
        "render_textplots_chart: {CHART_CHAR_WIDTH}x{CHART_CHAR_HEIGHT} char canvas ({CHART_PIXEL_WIDTH}x{CHART_PIXEL_HEIGHT} px), total series ({} points), done series ({} points), today_x={:.3}, x range=[0, {xmax:.3}], y range=[{ymin:.3}, {ymax:.3}]",
        total_series.len(),
        done_series.len(),
        geometry.today_x
    );

    let total_line_shape = Shape::Lines(&total_series);
    let done_line_shape = Shape::Lines(&done_series);
    let total_point_shape = Shape::Points(&total_series);
    let done_point_shape = Shape::Points(&done_series);
    let total_trend_shape = Shape::Lines(&total_trend_series);
    let done_trend_shape = Shape::Lines(&done_trend_series);
    let today_shape = Shape::Lines(&today_series);
    let mut chart =
        Chart::new_with_y_range(CHART_PIXEL_WIDTH, CHART_PIXEL_HEIGHT, 0.0, xmax, ymin, ymax);
    let mut chart_ref = &mut chart;
    chart_ref = chart_ref.y_label_format(LabelFormat::None);
    if let Some((start_label, end_label)) = x_axis_date_labels(points, geometry) {
        let split_x = xmax / 2.0;
        chart_ref = chart_ref.x_label_format(LabelFormat::Custom(Box::new(move |x| {
            if x <= split_x {
                start_label.clone()
            } else {
                end_label.clone()
            }
        })));
    }
    if color {
        if !total_trend_series.is_empty() {
            chart_ref = chart_ref.linecolorplot(&total_trend_shape, RGB8::new(255, 255, 0));
        }
        if !done_trend_series.is_empty() {
            chart_ref = chart_ref.linecolorplot(&done_trend_shape, RGB8::new(0, 255, 255));
        }
        chart_ref = chart_ref.linecolorplot(&today_shape, RGB8::new(255, 255, 255));
        chart_ref = chart_ref
            .linecolorplot(&total_line_shape, RGB8::new(255, 0, 0))
            .linecolorplot(&done_line_shape, RGB8::new(0, 255, 0))
            .linecolorplot(&total_point_shape, RGB8::new(255, 0, 0))
            .linecolorplot(&done_point_shape, RGB8::new(0, 255, 0));
    } else {
        // `textplots` only tells lines apart by color; without it every
        // series draws as the same plain Braille dot, so lines that
        // overlap become indistinguishable. `--ascii` is the recommended
        // way to keep the four lines distinguishable without color.
        if !total_trend_series.is_empty() {
            chart_ref = chart_ref.lineplot(&total_trend_shape);
        }
        if !done_trend_series.is_empty() {
            chart_ref = chart_ref.lineplot(&done_trend_shape);
        }
        chart_ref = chart_ref.lineplot(&today_shape);
        chart_ref = chart_ref
            .lineplot(&total_line_shape)
            .lineplot(&done_line_shape)
            .lineplot(&total_point_shape)
            .lineplot(&done_point_shape);
    }
    chart_ref.axis();
    chart_ref.figures();
    format!("{chart_ref}\n")
}
