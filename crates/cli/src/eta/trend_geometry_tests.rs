use super::*;

fn trend(slope_wtpd: f64, intercept_wt: f64) -> LinearTrend {
    LinearTrend {
        slope_wtpd,
        intercept_wt,
    }
}

#[test]
fn endpoints_at_x_zero_equal_intercept() {
    let t = trend(2.0, 5.0);
    let e = trend_line_endpoints(t, 10.0);
    assert_eq!(e.x0, 0.0);
    assert_eq!(e.y0, 5.0);
}

#[test]
fn endpoints_at_trend_end_x_apply_slope() {
    let t = trend(2.0, 5.0);
    let e = trend_line_endpoints(t, 10.0);
    assert_eq!(e.x1, 10.0);
    // y1 = slope * trend_end_x + intercept = 2*10 + 5 = 25
    assert_eq!(e.y1, 25.0);
}

#[test]
fn zero_slope_gives_flat_line() {
    let t = trend(0.0, 3.5);
    let e = trend_line_endpoints(t, 100.0);
    assert_eq!(e.y0, 3.5);
    assert_eq!(e.y1, 3.5);
}

#[test]
fn negative_slope_decreases_y() {
    let t = trend(-1.5, 10.0);
    let e = trend_line_endpoints(t, 4.0);
    assert_eq!(e.y0, 10.0);
    assert_eq!(e.y1, -1.5 * 4.0 + 10.0);
    assert!(e.y1 < e.y0);
}

#[test]
fn zero_trend_end_x_collapses_to_single_point() {
    let t = trend(3.0, 1.0);
    let e = trend_line_endpoints(t, 0.0);
    assert_eq!(e.x0, e.x1);
    assert_eq!(e.y0, e.y1);
}

#[test]
fn f32_variant_matches_f64_variant_rounded() {
    let t = trend(2.0, 1.0);
    let e = trend_line_endpoints(t, 10.0);
    let pts = trend_line_endpoints_f32(t, 10.0);
    assert_eq!(pts[0], (e.x0 as f32, e.y0 as f32));
    assert_eq!(pts[1], (e.x1 as f32, e.y1 as f32));
}

#[test]
fn f32_variant_has_two_points() {
    let t = trend(1.0, 1.0);
    let pts = trend_line_endpoints_f32(t, 5.0);
    assert_eq!(pts.len(), 2);
}
