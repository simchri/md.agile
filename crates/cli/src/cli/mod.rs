//! CLI for the `agile` binary.
//!
//! The library exposes this module so the binary can be a thin shim around
//! [`run`]. Subcommand-specific logic lives in [`subcommands`]; helpers shared
//! across subcommands live in [`common`].

use crate::config;
use crate::eta;
use clap::{Parser, Subcommand};
use log::error;
use std::path::Path;

pub mod common;
pub mod logger;
pub mod subcommands;

#[derive(Parser)]
#[command(
    name = "agile",
    about = "Plain-text, version-controlled task management",
    long_about = "\
Plain-text, version-controlled task management.

Run without a subcommand to open the next task in $VISUAL / $EDITOR."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Task operations
    #[command(visible_alias = "tasks")]
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Show recognised task files in priority order
    ///
    /// Prints one line per file: filename followed by its path relative
    /// to the current directory. Files are sorted alphabetically by
    /// filename only — their directory path does not affect ordering.
    /// This sort order is the global task priority order across files.
    #[command(visible_alias = "files")]
    File {
        /// Show only the first COUNT files
        #[arg(short = 'n', long, value_name = "COUNT")]
        next: Option<usize>,

        /// Show only the last COUNT files
        #[arg(long, value_name = "COUNT")]
        last: Option<usize>,
    },

    /// Validate task files against the built-in rule set
    ///
    /// Parses every *.agile.md file under the current directory and reports
    /// each issue as `<path>:<line>: <message>` on stdout. Exits with status 1
    /// if any issue is found, 0 if the project is clean.
    Check {
        /// Override the identity used for the E013 assignment/completion
        /// check with a literal `[Users.X]` key, instead of resolving it
        /// from `git config user.email`/`user.name`. Useful in CI, where the
        /// runner's git identity isn't the PR author's. A value that doesn't
        /// match any configured user is always treated as unauthorized for
        /// assigned tasks (never silently skipped).
        #[arg(long, value_name = "USER")]
        r#as: Option<String>,

        /// Override the git ref used as the "old" (last-known-good) side of
        /// the E013 assignment/completion diff, instead of the hard-coded
        /// `HEAD`. The "new" side remains the working directory. Useful in
        /// CI, where the checked-out code is already fully committed (so
        /// working-copy-vs-HEAD would show no diff at all) — pass the PR's
        /// base branch/commit instead (e.g. `origin/main`).
        #[arg(long, value_name = "REF")]
        base: Option<String>,
    },

    /// ETA and velocity reporting.
    ///
    /// With no flags, lists the ETA time span for every future milestone (in
    /// backlog order), one per line: `<span>   <milestone name>`.
    When {
        /// Show the current velocity and creep estimates, i.e. slopes of trend lines.
        #[arg(long)]
        velocity: bool,

        /// Show a terminal plot of total vs done weighted work over time.
        ///
        /// Defaults to `--next 1` (the next milestone) when `--next` is not given.
        #[arg(long, conflicts_with_all = ["velocity", "data"])]
        plot: bool,

        /// Output raw plot data points
        ///
        /// Defaults to `--next 1` (the next milestone) when `--next` is not given.
        #[arg(long, conflicts_with_all = ["velocity", "plot"])]
        data: bool,

        /// Restrict the historical data used for ETA/velocity calculations
        /// and plotting to given number of days before the milestones
        #[arg(
            long,
            value_name = "DAYS",
            value_parser = clap::value_parser!(u32).range(1..)
        )]
        last: Option<u32>,

        /// Extrapolate graph by given factor. Use to visualize trend
        /// lines beyond the range of recorded data (beyond today).
        /// Defaults to `1.3` (30% past the last data point, relative to
        /// the full historical span).
        ///
        /// Only valid with `--plot`.
        #[arg(
            long,
            value_name = "FACTOR",
            default_value_t = crate::eta::DEFAULT_EXTRA,
            requires = "plot",
            conflicts_with = "data"
        )]
        extra: f64,

        /// Render the plot using only 7-bit ASCII characters, use for terminals without Unicode/Braille support
        ///
        /// Only valid with `--plot`. Conflicts with `--html`.
        #[arg(long, requires = "plot", conflicts_with_all = ["data", "html"])]
        ascii: bool,

        /// Render the plot as a self-contained HTML file (inline SVG, no
        /// external dependencies) instead of printing a terminal chart.
        /// Written to "<milestone>-plot.html" in the current directory,
        /// where <milestone> is the milestone name lowercased and
        /// sanitized to `[a-z0-9_]`.
        ///
        /// Only valid with `--plot`. Conflicts with `--ascii`.
        #[arg(long, requires = "plot", conflicts_with_all = ["data", "ascii"])]
        html: bool,

        /// Disable ANSI color in the plot's chart, legend, and trend-line
        /// equations. With the default (Braille) chart, colors are the
        /// only way to tell the four data lines apart, so pair this with
        /// `--ascii` to keep them distinguishable by symbol instead.
        ///
        /// Only valid with `--plot`.
        #[arg(long, requires = "plot", conflicts_with = "data")]
        no_color: bool,

        /// Fit trend lines with plain (unweighted) ordinary least squares
        /// over the milestone's whole history, instead of the default
        /// exponential-decay recency-weighted fit.
        ///
        /// Conflicts with `--fit-algo-recent`, `--fit-algo-decay` and
        /// `--data` (which shows no fitted trend at all).
        #[arg(long, conflicts_with_all = ["fit_algo_recent", "fit_algo_decay", "data"])]
        fit_algo_linear: bool,

        /// Fit trend lines with the linear-rank recency-weighted algorithm
        /// (deduplicated to one point per day, then weighted by rank —
        /// oldest = 1, newest = N) instead of the default, more
        /// aggressively recency-biased exponential-decay fit.
        ///
        /// Conflicts with `--fit-algo-linear`, `--fit-algo-decay` and `--data`.
        #[arg(long, conflicts_with_all = ["fit_algo_linear", "fit_algo_decay", "data"])]
        fit_algo_recent: bool,

        /// Fit trend lines with the default exponential-decay
        /// recency-weighted algorithm — the same as omitting all
        /// `--fit-algo-*` flags. Only useful to make the default explicit.
        /// Points are deduplicated to one per day, then weighted so that
        /// weight decays exponentially with each point's age instead of
        /// ramping up linearly (as `--fit-algo-recent` does), so the most
        /// recent days can dominate the fit. The decay half-life scales
        /// with the milestone's own observed date span (20% of it), so the
        /// bias is relative rather than a fixed day count.
        ///
        /// Conflicts with `--fit-algo-linear`, `--fit-algo-recent` and
        /// `--data`.
        #[arg(long, conflicts_with_all = ["fit_algo_linear", "fit_algo_recent", "data"])]
        fit_algo_decay: bool,

        /// Select the Nth uncompleted milestone
        #[arg(long, value_name = "RANK")]
        next: Option<usize>,
    },

    /// Current-state milestone listing (no time estimation — see `when`)
    #[command(visible_alias = "milestone")]
    Milestones {
        /// Show detail for the Nth future milestone rank (as listed by the
        /// bare `agile milestones`) instead of the summary list.
        #[arg(long, value_name = "RANK")]
        next: Option<usize>,

        /// Show top-level task counts instead of task weight (the default).
        ///
        /// Only valid without `--next` (the detail view always shows both).
        #[arg(long, conflicts_with = "next")]
        count: bool,
    },

    /// Show currently closed tasks with completion date when known
    History,
}

#[derive(Subcommand)]
pub enum TaskAction {
    /// Show the next highest-priority incomplete task(s)
    ///
    /// With no arguments, returns the first incomplete ([ ]) top-level task
    /// across all task files in priority order, including its full subtask
    /// tree. Skips done ([x]) and cancelled ([-]) tasks. Prints nothing if
    /// every task is complete or cancelled.
    #[command(visible_alias = "show")]
    Next {
        /// A plain count (e.g. `3`) to show the next N incomplete top-level
        /// tasks, or a dotted address (e.g. `1.2`, `2.1.4`) to show one
        /// specific (sub)task by position: the first number selects the Nth
        /// still-incomplete top-level task (in priority order); each
        /// subsequent number selects the Nth direct child of the
        /// previously-selected node (in document order, any status).
        /// Omit to show just the single next task.
        address: Option<String>,

        /// Only show task(s) that are unassigned (open to anyone) or
        /// assigned to me, directly or via a group — the same eligibility
        /// rule as the E013 assignment/completion check. Can be combined
        /// with a plain count (e.g. `next 3 --mine`), but not with a
        /// dotted address.
        #[arg(long)]
        mine: bool,

        /// Resolve `--mine` as this literal `[Users.X]` config key instead
        /// of the git identity from `git config user.email`/`user.name`.
        /// Implies `--mine` even when `--mine` isn't given.
        #[arg(long, value_name = "USER")]
        r#as: Option<String>,

        /// Also show each (sub)task's body text alongside its title
        #[arg(long)]
        full: bool,

        /// Disable ANSI bold/color output. The concrete next actionable
        /// line is instead marked by appending " <==" to it in plain text.
        #[arg(long)]
        no_markup: bool,
    },

    /// Show the most recently completed task(s)
    ///
    /// The mirror image of `agile task next`: walks *closed* top-level
    /// tasks (any with `Done`/`Cancelled` work in them — including
    /// partially-completed ones) in reverse priority order, so the most
    /// recently touched top-level task is address `1`, the one before that
    /// `2`, and so on. With no arguments, shows just that single most
    /// recent one, including its full subtree — exactly the address scheme
    /// `agile task undone` now accepts, including whole fully-done
    /// top-level tasks.
    #[command(visible_alias = "prev")]
    Previous {
        /// A plain count (e.g. `3`, meaning "3rd most recently touched
        /// top-level task") or a dotted address (e.g. `1.2`) to show one
        /// specific (sub)task by position, using the same reverse-rank
        /// numbering as with no address. Omit to show just the single most
        /// recently touched top-level task.
        address: Option<String>,

        /// Also show each (sub)task's body text alongside its title
        #[arg(long)]
        full: bool,

        /// Disable ANSI bold/color output. The most recently completed line
        /// is instead marked by appending " <==" to it in plain text.
        #[arg(long)]
        no_markup: bool,
    },

    /// Mark the (sub)task at ADDRESS done
    ///
    /// ADDRESS uses the same scheme as `agile task next`'s dotted address:
    /// e.g. `2` for the 2nd still-incomplete top-level task, or `1.3` for
    /// its 3rd direct child. Refuses (printing the violated rule instead of
    /// writing the file) if marking the node done would leave it with
    /// incomplete required children, missing required subtasks, a
    /// disallowed cancelled required subtask, or would be an unauthorized
    /// completion of a task assigned to someone else (E013). Only reads/
    /// writes the one file the addressed task lives in — it doesn't
    /// re-validate the rest of the project.
    Done {
        /// Dotted address, e.g. `2` or `1.3`.
        address: String,

        /// Override the identity used for the E013 assignment/completion
        /// check instead of the live git identity — a literal `[Users.X]`
        /// config key (e.g. `--as alice`), not a git name/email. A key that
        /// doesn't match any configured user is always treated as
        /// unauthorized for assigned tasks (never silently skipped).
        #[arg(long, value_name = "USER")]
        r#as: Option<String>,
    },

    /// Revert the (sub)task at ADDRESS back to todo
    ///
    /// The inverse of `agile task done`: always succeeds if the addressed
    /// node is currently done (`[x]`) — there are no completion rules to
    /// satisfy in reverse. ADDRESS uses the same "reverse rank" resolution
    /// as `agile task previous`: the first segment selects the Nth
    /// top-level task counting back from the last one with any closed
    /// (`Done`/`Cancelled`) work in it — so a whole already fully-done
    /// top-level task is reachable this way, not just a still-open subtask
    /// of an otherwise incomplete parent. Every subsequent segment selects
    /// the Nth direct child (any status) from there.
    Undone {
        /// Dotted address, e.g. `2` or `1.3`.
        address: String,
    },

    /// List tasks in priority order
    ///
    /// Without arguments, prints every incomplete (`[ ]`) top-level task
    /// from all *.agile.md files in priority order, each with its full
    /// subtask tree. Each task is shown with its status marker ([ ] todo,
    /// [x] done, [-] cancelled) and subtasks indented by two spaces per
    /// level.
    List {
        /// A 1-based, inclusive range over the top-level tasks that would
        /// otherwise be shown (respecting `--all`/`--mine`), e.g. `2:4`
        /// shows the 2nd through 4th such tasks (each with its own
        /// subtree). Takes precedence over `--next`/`--last` if given.
        range: Option<String>,

        /// Show only the first COUNT entries
        #[arg(short = 'n', long, value_name = "COUNT")]
        next: Option<usize>,

        /// Show only the last COUNT entries
        #[arg(long, value_name = "COUNT")]
        last: Option<usize>,

        /// Show all tasks including done and cancelled
        #[arg(short = 'a', long)]
        all: bool,

        /// Only list top-level tasks that are unassigned or assigned to me
        /// (directly or via a group) — same eligibility rule as the E013
        /// assignment/completion check and `agile task next --mine`.
        #[arg(long)]
        mine: bool,

        /// Resolve `--mine` as this literal `[Users.X]` config key instead
        /// of the git identity from `git config user.email`/`user.name`.
        /// Implies `--mine` even when `--mine` isn't given.
        #[arg(long, value_name = "USER")]
        r#as: Option<String>,
    },
}

/// Parses CLI arguments and dispatches to the matching subcommand.
///
/// This is the entry point used by `src/main.rs`. It is the only function the
/// binary needs to call.
pub fn run() {
    log::debug!("agile cli run()");

    let root = Path::new(".");
    let config = match config::Config::load(root) {
        Ok(c) => c,
        Err(e) => {
            error!("{e}");
            std::process::exit(1);
        }
    };

    match Cli::parse().command {
        None => subcommands::default::run(root),
        Some(Command::File { next, last }) => {
            subcommands::list::run_files(root, next, last);
        }
        Some(Command::Task {
            action:
                TaskAction::List {
                    range,
                    next,
                    last,
                    all,
                    mine,
                    r#as,
                },
        }) => {
            subcommands::list::run_tasks(
                root,
                &config,
                next,
                last,
                all,
                mine,
                r#as.as_deref(),
                range.as_deref(),
            );
        }
        Some(Command::Task {
            action:
                TaskAction::Next {
                    address,
                    mine,
                    r#as,
                    full,
                    no_markup,
                },
        }) => {
            subcommands::task::run_next(
                root,
                &config,
                address.as_deref(),
                mine,
                r#as.as_deref(),
                full,
                no_markup,
            );
        }
        Some(Command::Task {
            action: TaskAction::Done { address, r#as },
        }) => {
            subcommands::task::run_done(root, &config, &address, r#as.as_deref());
        }
        Some(Command::Task {
            action: TaskAction::Undone { address },
        }) => {
            subcommands::task::run_undone(root, &config, &address);
        }
        Some(Command::Task {
            action:
                TaskAction::Previous {
                    address,
                    full,
                    no_markup,
                },
        }) => {
            subcommands::task::run_previous(root, &config, address.as_deref(), full, no_markup);
        }
        Some(Command::Check { r#as, base }) => {
            subcommands::check::run(root, &config, r#as.as_deref(), base.as_deref());
        }
        Some(Command::When {
            velocity,
            plot,
            data,
            extra,
            ascii,
            html,
            no_color,
            fit_algo_linear,
            fit_algo_recent,
            fit_algo_decay: _,
            last,
            next,
        }) => {
            let algorithm = if fit_algo_linear {
                eta::TrendFitAlgorithm::OrdinaryLeastSquares
            } else if fit_algo_recent {
                eta::TrendFitAlgorithm::RecencyWeighted
            } else {
                eta::TrendFitAlgorithm::ExponentialDecay
            };
            subcommands::when::run(
                root, &config, next, velocity, plot, data, extra, ascii, html, no_color, last,
                algorithm,
            );
        }
        Some(Command::History) => {
            subcommands::history::run(root);
        }
        Some(Command::Milestones { next, count }) => {
            subcommands::milestones::run(root, next, count);
        }
    }
}

#[cfg(test)]
mod tests;
