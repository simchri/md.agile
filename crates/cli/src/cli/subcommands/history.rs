//! `agile history` — list currently closed tasks with completion dates.

use crate::cli::common::{find_task_files, parse_file};
use crate::git;
use crate::parser::{self, FileItem, Status};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NodeKey {
    path: Vec<String>,
    occurrence: usize,
}

#[derive(Debug, Clone)]
struct FlatNode {
    key: NodeKey,
    status: Status,
    indent: usize,
    title: String,
}

/// `agile history` entry point.
pub fn run(root: &Path) {
    let mut out = String::new();
    for file in find_task_files(root) {
        let items = parse_file(&file);
        let current_nodes = flatten_nodes(&items);
        if current_nodes.is_empty() {
            continue;
        }

        let completion_dates = completion_dates_for_file(root, &file);
        for node in current_nodes
            .iter()
            .filter(|n| matches!(n.status, Status::Done | Status::Cancelled))
        {
            let date = completion_dates
                .get(&node.key)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            out.push_str(&render_history_line(&date, node));
        }
    }
    print!("{out}");
}

fn render_history_line(date: &str, node: &FlatNode) -> String {
    format!(
        "{date} {}- {} {}\n",
        " ".repeat(node.indent),
        status_marker(&node.status),
        node.title
    )
}

fn status_marker(status: &Status) -> &'static str {
    match status {
        Status::Todo => "[ ]",
        Status::Done => "[x]",
        Status::Cancelled => "[-]",
    }
}

fn completion_dates_for_file(root: &Path, relative_path: &Path) -> HashMap<NodeKey, String> {
    if !git::is_git_repo(root) {
        return HashMap::new();
    }

    let mut completion_dates = HashMap::new();
    let mut commits = git::commits_touching_path(root, relative_path);
    if commits.len() < 2 {
        return completion_dates;
    }

    // Walk oldest -> newest to keep the latest transition-to-closed date.
    commits.reverse();
    for pair in commits.windows(2) {
        let old = &pair[0];
        let new = &pair[1];
        let Some(old_content) = git::file_content_at_ref(root, &old.sha, relative_path) else {
            continue;
        };
        let Some(new_content) = git::file_content_at_ref(root, &new.sha, relative_path) else {
            continue;
        };

        let old_items = parser::parse(&old_content, relative_path.to_path_buf());
        let new_items = parser::parse(&new_content, relative_path.to_path_buf());
        let old_nodes = flatten_nodes(&old_items);
        let new_nodes = flatten_nodes(&new_items);

        let old_status_by_key: HashMap<NodeKey, Status> = old_nodes
            .into_iter()
            .map(|n| (n.key, n.status))
            .collect::<HashMap<_, _>>();

        let date = unix_to_yyyy_mm_dd(new.timestamp);
        for node in new_nodes {
            let was_closed = old_status_by_key
                .get(&node.key)
                .is_some_and(|s| is_closed(s));
            if !was_closed && is_closed(&node.status) {
                completion_dates.insert(node.key, date.clone());
            }
        }
    }

    completion_dates
}

fn flatten_nodes(items: &[FileItem]) -> Vec<FlatNode> {
    let mut raw = Vec::new();
    for item in items {
        let FileItem::Task(task) = item else {
            continue;
        };
        let path = vec![task.title.clone()];
        raw.push((
            path.clone(),
            task.status.clone(),
            task.indent,
            task.title.clone(),
        ));
        flatten_subtasks(&mut raw, &path, &task.children);
    }

    let mut occurrence_index: HashMap<Vec<String>, usize> = HashMap::new();
    raw.into_iter()
        .map(|(path, status, indent, title)| {
            let occurrence = occurrence_index.entry(path.clone()).or_insert(0);
            let key = NodeKey {
                path,
                occurrence: *occurrence,
            };
            *occurrence += 1;
            FlatNode {
                key,
                status,
                indent,
                title,
            }
        })
        .collect()
}

fn flatten_subtasks(
    out: &mut Vec<(Vec<String>, Status, usize, String)>,
    parent_path: &[String],
    children: &[parser::Subtask],
) {
    for child in children {
        let mut path = parent_path.to_vec();
        path.push(child.title.clone());
        out.push((
            path.clone(),
            child.status.clone(),
            child.indent,
            child.title.clone(),
        ));
        flatten_subtasks(out, &path, &child.children);
    }
}

fn is_closed(status: &Status) -> bool {
    matches!(status, Status::Done | Status::Cancelled)
}

fn unix_to_yyyy_mm_dd(unix_ts: i64) -> String {
    let days = unix_ts.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

// Converts days since Unix epoch to a Gregorian date in UTC.
fn civil_from_days(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = mp + if mp < 10 { 3 } else { -9 }; // [1, 12]
    if m <= 2 {
        y += 1;
    }
    (y, m, d)
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
