//! Commit-by-commit cache for task history snapshots.

use crate::git;
use crate::parser::{self, FileItem, Status, Subtask, Task};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HistoryCache {
    pub entries: Vec<HistoryCacheEntry>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HistoryCacheEntry {
    pub commit_hash: String,
    pub commit_date: String,
    pub open_tasks_count: usize,
    pub open_tasks_weight: f64,
    pub done_tasks_count: usize,
    pub done_tasks_weight: f64,
}

pub fn update(root: &Path) -> Option<HistoryCache> {
    if !git::is_git_repo(root) {
        return None;
    }
    let cache_path = cache_file_path(root)?;
    let mut cache = read_cache_file(&cache_path).unwrap_or(HistoryCache { entries: vec![] });

    let mut commits = git::commits(root);
    if commits.is_empty() {
        return Some(cache);
    }
    commits.reverse(); // oldest -> newest

    let common_prefix_len = cache
        .entries
        .iter()
        .zip(commits.iter())
        .take_while(|(entry, commit)| entry.commit_hash == commit.sha)
        .count();
    cache.entries.truncate(common_prefix_len);

    for commit in commits.iter().skip(common_prefix_len) {
        let entry = compute_entry(root, &commit.sha, commit.timestamp);
        cache.entries.push(entry);
    }

    if write_cache_file(&cache_path, &cache).is_err() {
        return None;
    }
    Some(cache)
}

fn compute_entry(root: &Path, commit_hash: &str, timestamp: i64) -> HistoryCacheEntry {
    let mut open_tasks_count = 0usize;
    let mut open_tasks_weight = 0.0f64;
    let mut done_tasks_count = 0usize;
    let mut done_tasks_weight = 0.0f64;

    for path in git::task_files_at_ref(root, commit_hash) {
        let Some(content) = git::file_content_at_ref(root, commit_hash, &path) else {
            continue;
        };
        let items = parser::parse(&content, path);
        for item in items {
            let FileItem::Task(task) = item else {
                continue;
            };
            let weight = task_weight(&task);
            match task.status {
                Status::Todo => {
                    open_tasks_count += 1;
                    open_tasks_weight += weight;
                }
                Status::Done | Status::Cancelled => {
                    done_tasks_count += 1;
                    done_tasks_weight += weight;
                }
            }
        }
    }

    HistoryCacheEntry {
        commit_hash: commit_hash.to_string(),
        commit_date: unix_to_yyyy_mm_dd(timestamp),
        open_tasks_count,
        open_tasks_weight,
        done_tasks_count,
        done_tasks_weight,
    }
}

fn task_weight(task: &Task) -> f64 {
    1.0 + subtasks_weight(&task.children, 2)
}

fn subtasks_weight(subtasks: &[Subtask], depth: usize) -> f64 {
    subtasks
        .iter()
        .map(|s| (1.0 / depth as f64) + subtasks_weight(&s.children, depth + 1))
        .sum()
}

fn cache_file_path(root: &Path) -> Option<std::path::PathBuf> {
    Some(
        git::git_dir(root)?
            .join("mdagile")
            .join("history-cache.json"),
    )
}

fn read_cache_file(path: &Path) -> Option<HistoryCache> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_cache_file(path: &Path, cache: &HistoryCache) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(cache)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, format!("{payload}\n"))
}

fn unix_to_yyyy_mm_dd(unix_ts: i64) -> String {
    let days = unix_ts.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    if m <= 2 {
        y += 1;
    }
    (y, m, d)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
