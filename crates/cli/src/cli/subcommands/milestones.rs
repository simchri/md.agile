//! `agile milestones` — current-state milestone listing (no time
//! estimation; see `when` for ETA/velocity reporting).

use crate::eta;
use std::path::Path;

/// `agile milestones` entry point.
///
/// With no flags, lists every future milestone (in backlog order, ranked
/// from 1) with its done/total weighted counts and percentage complete.
/// `--count` shows top-level task counts instead of weight. `--next
/// <rank>` shows a detail breakdown (task counts and weight) for one
/// future milestone instead of the summary list.
pub fn run(root: &Path, next: Option<usize>, count: bool) {
    if let Some(rank) = next {
        match eta::build_milestone_detail_report(root, rank) {
            Ok(report) => print!("{report}"),
            Err(msg) => {
                log::error!("{msg}");
                std::process::exit(1);
            }
        }
        return;
    }

    print!("{}", eta::build_milestones_list_report(root, count));
}
