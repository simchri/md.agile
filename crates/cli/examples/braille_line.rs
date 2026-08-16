//! Minimal sandbox for playing with the `textplots` (Braille) charting
//! library, independent of the rest of the `agile` codebase.
//!
//! Draws a single straight line `y = intercept + slope * x` over `x` in
//! `0.0..=10.0`, with `intercept = 1.0` and `slope = 2.0`. Tweak the
//! constants below, or the `Chart`/`Shape`/color calls, and re-run to see
//! how `textplots` behaves.
//!
//! Run with:
//!   cargo run --example braille_line -p mdagile

use rgb::RGB8;
use textplots::{Chart, ColorPlot, Shape};

const INTERCEPT: f32 = 2.0;
const SLOPE: f32 = -2.0;
const X_MIN: f32 = 0.0;
const X_MAX: f32 = 5.0;

const INTERCEPT_2: f32 = -2.0;
const SLOPE_2: f32 = 2.0;
const X_MIN_2: f32 = 0.0;
const X_MAX_2: f32 = 5.0;

fn main() {
    // A line only needs two points; textplots draws a straight segment
    // between them for `Shape::Lines`.
    let points = [
        (X_MIN, INTERCEPT + SLOPE * X_MIN),
        (X_MAX, INTERCEPT + SLOPE * X_MAX),
    ];
    let line = Shape::Lines(&points);

    let points_2 = [
        (X_MIN_2, INTERCEPT_2 + SLOPE_2 * X_MIN_2),
        (X_MAX_2, INTERCEPT_2 + SLOPE_2 * X_MAX_2),
    ];
    let line_2 = Shape::Lines(&points_2);

    let mut chart = Chart::new(120, 80, X_MIN, X_MAX);
    let mut chart_ref = chart.linecolorplot(&line, RGB8::new(0, 255, 255));
    chart_ref = chart_ref.linecolorplot(&line_2, RGB8::new(0, 255, 255));
    chart_ref.axis();
    chart_ref.figures();

    println!("{chart_ref}");
}
