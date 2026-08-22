//! ETA/velocity computation primitives.
//!
//! Split by concern:
//! - [`velocity`] — velocity/creep estimation and status-transition tracking
//! - [`plot_data`] — building [`TodoDonePlot`] data and the geometry/sampling
//!   shared by every chart renderer
//! - [`trend`] — trend-line fitting
//! - [`eta_math`] — ETA math (intersecting trend lines into a target date)
//! - [`eta_text`] — ETA text formatting/rendering
//! - [`chart_common`] — rendering pieces shared by the terminal and HTML charts
//! - [`chart_terminal`] — the terminal chart output (Braille or `--ascii`)
//! - [`chart_html`] — the `--html` SVG chart output
//! - [`data_dump`] — the `--data` raw table output
//! - [`report`] — the bare `agile when` list and `--velocity` text reports
//! - [`date_utils`] — shared unix-days/calendar-date conversions
//! - [`trend_geometry`] — shared trend-line-to-line-segment geometry
//! - [`milestone_stats`] — git-independent current-state milestone stats,
//!   used by `agile milestones`
//! - [`milestone_report`] — `agile milestones`' list/detail text reports

mod chart_common;
mod chart_html;
mod chart_terminal;
mod chart_trends;
mod data_dump;
mod date_utils;
mod eta_math;
mod eta_text;
mod milestone_report;
mod milestone_stats;
mod plot_data;
mod report;
mod trend;
mod trend_geometry;
mod velocity;

pub use chart_html::write_todo_done_plot_html;
pub use chart_terminal::render_todo_done_plot;
pub use data_dump::render_todo_done_data;
pub use milestone_report::{build_milestone_detail_report, build_milestones_list_report};
pub use plot_data::{
    DEFAULT_EXTRA, TodoDonePlot, TodoDonePlotPoint, build_todo_done_plot, restrict_to_window_days,
};
pub use report::{build_when_report, render_velocity_text};
pub use trend::TrendFitAlgorithm;
pub(crate) use velocity::weight_for_depth;
pub use velocity::{
    StatusTransition, TransitionKey, VelocityEstimate, estimate_velocity,
    estimate_velocity_with_window, status_transitions,
};
