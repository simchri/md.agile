//! Raw data dump output: `agile when --plot --data`, a plain table of task
//! counts and weights (no trend line fitting), one row per plot point.

use super::plot_data::TodoDonePlot;

/// Renders the raw plot data (task counts and weights, no trend line
/// fitting) as a simple table, one row per point.
pub fn render_todo_done_data(plot: &TodoDonePlot) -> String {
    log::debug!(
        "render_todo_done_data: milestone={:?}, {} rows (Date, Total, Done, Total Wt, Done Wt)",
        plot.milestone_name,
        plot.points.len()
    );
    let mut out = String::new();
    out.push_str(&format!("Milestone: {}\n\n", plot.milestone_name));
    out.push_str(&format!(
        "{:<12}{:>7}{:>7}{:>10}{:>9}\n",
        "Date", "Total", "Done", "Total Wt", "Done Wt"
    ));
    for point in &plot.points {
        out.push_str(&format!(
            "{:<12}{:>7}{:>7}{:>10.2}{:>9.2}\n",
            point.date,
            point.total_top_level_t,
            point.done_top_level_t,
            point.total_weight_wt,
            point.done_weight_wt
        ));
    }
    out
}

#[cfg(test)]
#[path = "data_dump_tests.rs"]
mod tests;
