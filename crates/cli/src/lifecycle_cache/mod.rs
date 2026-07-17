//! Per-entity lifecycle cache used by `agile history`.

use crate::eta::TransitionKey;
use crate::git;
use crate::parser::{self, FileItem, Status};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CACHE_VERSION: u32 = 1;
const MATCH_THRESHOLD: f64 = 0.70;
const MATCH_TIE_DELTA: f64 = 0.05;
const TITLE_WEIGHT: f64 = 0.50;
const PARENT_WEIGHT: f64 = 0.25;
const DEPTH_WEIGHT: f64 = 0.15;
const POSITION_WEIGHT: f64 = 0.10;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LifecycleCache {
    pub version: u32,
    pub head_commit: String,
    pub commit_chain: Vec<String>, // oldest -> newest
    pub entities: HashMap<String, CachedEntity>,
    pub files: HashMap<String, FileHeadState>,
    pub spans: Vec<CommitSpan>,
    pub next_entity_id: u64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CachedEntity {
    pub first_seen_commit: String,
    pub last_seen_commit: String,
    pub latest_status: CachedStatus,
    pub latest_close_date: Option<String>,
    pub fingerprint: Fingerprint,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fingerprint {
    pub title: String,
    pub depth: usize,
    pub parent_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileHeadState {
    pub entities: Vec<FileEntityNode>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileEntityNode {
    pub entity_id: String,
    pub key: KeyRepr,
    pub title: String,
    pub status: CachedStatus,
    pub depth: usize,
    pub parent_title: Option<String>,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct KeyRepr {
    pub path: Vec<String>,
    pub occurrence: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommitSpan {
    pub old_commit: String,
    pub new_commit: String,
    pub new_commit_date: String,
    pub transitions: Vec<EntityTransition>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EntityTransition {
    pub entity_id: String,
    pub old_status: Option<CachedStatus>,
    pub new_status: CachedStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CachedStatus {
    Todo,
    Done,
    Cancelled,
}

#[derive(Debug, Clone)]
struct FlatNode {
    key: TransitionKey,
    status: Status,
    title: String,
    depth: usize,
    parent_title: Option<String>,
    position: usize,
}

pub fn update(root: &Path) -> Option<LifecycleCache> {
    if !git::is_git_repo(root) {
        return None;
    }
    let cache_path = cache_file_path(root)?;
    let mut commits = git::commits(root);
    if commits.is_empty() {
        return None;
    }
    commits.reverse(); // oldest -> newest
    let commit_chain: Vec<String> = commits.iter().map(|c| c.sha.clone()).collect();
    let head_commit = commit_chain.last()?.clone();

    if let Some(cache) = read_cache_file(&cache_path).filter(|c| c.version == CACHE_VERSION) {
        if cache.commit_chain == commit_chain {
            return Some(cache);
        }
        if commit_chain.starts_with(&cache.commit_chain) {
            let extended = append_new_commits(root, cache, &commits)?;
            if write_cache_file(&cache_path, &extended).is_err() {
                return None;
            }
            return Some(extended);
        }
    }

    let rebuilt = rebuild_from_scratch(root, &commits, &head_commit)?;
    if write_cache_file(&cache_path, &rebuilt).is_err() {
        return None;
    }
    Some(rebuilt)
}

pub fn completion_dates_for_current_file(
    root: &Path,
    relative_path: &Path,
    current_items: &[FileItem],
) -> HashMap<TransitionKey, String> {
    let Some(cache) = update(root) else {
        return HashMap::new();
    };
    let normalized_path = normalize_repo_path(root, relative_path);
    let Some(file_state) = cache.files.get(&path_key(&normalized_path)) else {
        return HashMap::new();
    };

    let current_nodes = flatten_nodes(current_items);
    let head_nodes = file_state
        .entities
        .iter()
        .enumerate()
        .map(|(idx, n)| HeadNodeView {
            index: idx,
            node: n,
            used: false,
        })
        .collect::<Vec<_>>();
    let assignments = assign_current_to_head(&head_nodes, &current_nodes);

    let mut out = HashMap::new();
    for (idx, current) in current_nodes.iter().enumerate() {
        if !is_closed(&current.status) {
            continue;
        }
        let Some(entity_id) = assignments.get(&idx) else {
            continue;
        };
        let Some(entity) = cache.entities.get(entity_id) else {
            continue;
        };
        if !is_cached_closed(entity.latest_status) {
            continue;
        }
        let Some(date) = &entity.latest_close_date else {
            continue;
        };
        out.insert(current.key.clone(), date.clone());
    }
    out
}

fn append_new_commits(
    root: &Path,
    mut cache: LifecycleCache,
    commits: &[git::CommitRef],
) -> Option<LifecycleCache> {
    let mut prev_files = cache.files.clone();
    let mut prev_commit = cache.head_commit.clone();
    for commit in commits.iter().skip(cache.commit_chain.len()) {
        let (new_files, transitions, next_id) = advance_commit(
            root,
            &prev_files,
            &prev_commit,
            &commit.sha,
            cache.next_entity_id,
            &mut cache.entities,
        )?;
        cache.next_entity_id = next_id;
        let date = unix_to_yyyy_mm_dd(commit.timestamp);
        apply_transitions(&mut cache.entities, &commit.sha, &date, &transitions);
        cache.spans.push(CommitSpan {
            old_commit: prev_commit.clone(),
            new_commit: commit.sha.clone(),
            new_commit_date: date,
            transitions,
        });
        cache.commit_chain.push(commit.sha.clone());
        cache.head_commit = commit.sha.clone();
        prev_commit = commit.sha.clone();
        prev_files = new_files;
    }
    cache.files = prev_files;
    Some(cache)
}

fn rebuild_from_scratch(
    root: &Path,
    commits: &[git::CommitRef],
    head_commit: &str,
) -> Option<LifecycleCache> {
    let mut entities: HashMap<String, CachedEntity> = HashMap::new();
    let mut next_entity_id = 1u64;
    let first_commit = commits.first()?;
    let mut files = files_for_commit(
        root,
        &first_commit.sha,
        &HashMap::new(),
        &mut next_entity_id,
        &mut entities,
    );

    for commit in commits.iter().skip(1) {
        let prev_commit =
            &commits[commits.iter().position(|c| c.sha == commit.sha).unwrap() - 1].sha;
        let (new_files, transitions, new_next_id) = advance_commit(
            root,
            &files,
            prev_commit,
            &commit.sha,
            next_entity_id,
            &mut entities,
        )?;
        let date = unix_to_yyyy_mm_dd(commit.timestamp);
        apply_transitions(&mut entities, &commit.sha, &date, &transitions);
        next_entity_id = new_next_id;
        files = new_files;
    }

    let spans = build_spans(root, commits, &mut entities)?;

    Some(LifecycleCache {
        version: CACHE_VERSION,
        head_commit: head_commit.to_string(),
        commit_chain: commits.iter().map(|c| c.sha.clone()).collect(),
        entities,
        files,
        spans,
        next_entity_id,
    })
}

fn build_spans(
    root: &Path,
    commits: &[git::CommitRef],
    entities: &mut HashMap<String, CachedEntity>,
) -> Option<Vec<CommitSpan>> {
    if commits.len() < 2 {
        return Some(Vec::new());
    }
    let mut spans = Vec::new();
    let mut next_entity_id = entities
        .keys()
        .filter_map(|k| k.strip_prefix('e'))
        .filter_map(|n| n.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    let mut files = files_for_commit(
        root,
        &commits[0].sha,
        &HashMap::new(),
        &mut next_entity_id,
        entities,
    );
    for pair in commits.windows(2) {
        let old_commit = &pair[0].sha;
        let new_commit = &pair[1].sha;
        let (new_files, transitions, new_next_id) = advance_commit(
            root,
            &files,
            old_commit,
            new_commit,
            next_entity_id,
            entities,
        )?;
        let date = unix_to_yyyy_mm_dd(pair[1].timestamp);
        apply_transitions(entities, new_commit, &date, &transitions);
        spans.push(CommitSpan {
            old_commit: old_commit.clone(),
            new_commit: new_commit.clone(),
            new_commit_date: date,
            transitions,
        });
        files = new_files;
        next_entity_id = new_next_id;
    }
    Some(spans)
}

fn files_for_commit(
    root: &Path,
    commit_sha: &str,
    previous: &HashMap<String, FileHeadState>,
    next_entity_id: &mut u64,
    entities: &mut HashMap<String, CachedEntity>,
) -> HashMap<String, FileHeadState> {
    let mut out = HashMap::new();
    for path in git::task_files_at_ref(root, commit_sha) {
        let path_s = path_key(&path);
        let Some(content) = git::file_content_at_ref(root, commit_sha, &path) else {
            continue;
        };
        let items = parser::parse(&content, path);
        let nodes = flatten_nodes(&items);
        let prev = previous
            .get(&path_s)
            .cloned()
            .unwrap_or(FileHeadState { entities: vec![] });
        out.insert(
            path_s,
            assign_entities(&prev.entities, &nodes, commit_sha, next_entity_id, entities),
        );
    }
    out
}

fn advance_commit(
    root: &Path,
    previous_files: &HashMap<String, FileHeadState>,
    old_commit: &str,
    new_commit: &str,
    mut next_entity_id: u64,
    entities: &mut HashMap<String, CachedEntity>,
) -> Option<(HashMap<String, FileHeadState>, Vec<EntityTransition>, u64)> {
    let old_paths: HashSet<String> = git::task_files_at_ref(root, old_commit)
        .into_iter()
        .map(|p| path_key(&p))
        .collect();
    let new_paths: HashSet<String> = git::task_files_at_ref(root, new_commit)
        .into_iter()
        .map(|p| path_key(&p))
        .collect();
    let all_paths: HashSet<String> = old_paths.union(&new_paths).cloned().collect();

    let mut new_files = HashMap::new();
    let mut transitions = Vec::new();
    for path in all_paths {
        let prev_state = previous_files
            .get(&path)
            .cloned()
            .unwrap_or(FileHeadState { entities: vec![] });
        let new_items = load_items_at_ref(root, new_commit, &path);
        let nodes = flatten_nodes(&new_items);
        let state = assign_entities(
            &prev_state.entities,
            &nodes,
            new_commit,
            &mut next_entity_id,
            entities,
        );
        transitions.extend(file_transitions(&prev_state.entities, &state.entities));
        if !state.entities.is_empty() {
            new_files.insert(path, state);
        }
    }
    Some((new_files, transitions, next_entity_id))
}

fn file_transitions(
    old_nodes: &[FileEntityNode],
    new_nodes: &[FileEntityNode],
) -> Vec<EntityTransition> {
    let old_status: HashMap<String, CachedStatus> = old_nodes
        .iter()
        .map(|n| (n.entity_id.clone(), n.status))
        .collect();
    new_nodes
        .iter()
        .map(|n| EntityTransition {
            entity_id: n.entity_id.clone(),
            old_status: old_status.get(&n.entity_id).copied(),
            new_status: n.status,
        })
        .collect()
}

fn assign_entities(
    old_nodes: &[FileEntityNode],
    new_nodes: &[FlatNode],
    commit_sha: &str,
    next_entity_id: &mut u64,
    entities: &mut HashMap<String, CachedEntity>,
) -> FileHeadState {
    let assignments = assign_current_to_head(
        &old_nodes
            .iter()
            .enumerate()
            .map(|(idx, node)| HeadNodeView {
                index: idx,
                node,
                used: false,
            })
            .collect::<Vec<_>>(),
        new_nodes,
    );
    let mut out = Vec::with_capacity(new_nodes.len());
    for (idx, node) in new_nodes.iter().enumerate() {
        let entity_id = assignments
            .get(&idx)
            .cloned()
            .unwrap_or_else(|| new_entity_id(next_entity_id));
        let status = cached_status_from(&node.status);
        out.push(FileEntityNode {
            entity_id: entity_id.clone(),
            key: KeyRepr {
                path: node.key.path.clone(),
                occurrence: node.key.occurrence,
            },
            title: node.title.clone(),
            status,
            depth: node.depth,
            parent_title: node.parent_title.clone(),
            position: node.position,
        });
        entities
            .entry(entity_id.clone())
            .and_modify(|e| {
                e.last_seen_commit = commit_sha.to_string();
                e.latest_status = status;
                e.fingerprint = Fingerprint {
                    title: node.title.clone(),
                    depth: node.depth,
                    parent_title: node.parent_title.clone(),
                };
            })
            .or_insert(CachedEntity {
                first_seen_commit: commit_sha.to_string(),
                last_seen_commit: commit_sha.to_string(),
                latest_status: status,
                latest_close_date: None,
                fingerprint: Fingerprint {
                    title: node.title.clone(),
                    depth: node.depth,
                    parent_title: node.parent_title.clone(),
                },
            });
    }
    FileHeadState { entities: out }
}

struct HeadNodeView<'a> {
    index: usize,
    node: &'a FileEntityNode,
    used: bool,
}

fn assign_current_to_head(
    old_nodes: &[HeadNodeView<'_>],
    new_nodes: &[FlatNode],
) -> HashMap<usize, String> {
    let mut assignments = HashMap::new();
    let mut used_old = HashSet::<usize>::new();

    let old_by_key: HashMap<KeyRepr, &FileEntityNode> = old_nodes
        .iter()
        .map(|v| (v.node.key.clone(), v.node))
        .collect();
    for (idx, current) in new_nodes.iter().enumerate() {
        let key = KeyRepr {
            path: current.key.path.clone(),
            occurrence: current.key.occurrence,
        };
        if let Some(node) = old_by_key.get(&key) {
            assignments.insert(idx, node.entity_id.clone());
            if let Some(v) = old_nodes
                .iter()
                .find(|v| v.node.entity_id == node.entity_id)
            {
                used_old.insert(v.index);
            }
        }
    }

    for (idx, current) in new_nodes.iter().enumerate() {
        if assignments.contains_key(&idx) {
            continue;
        }
        let mut candidates: Vec<(usize, f64, String)> = old_nodes
            .iter()
            .filter(|o| !used_old.contains(&o.index) && !o.used)
            .map(|o| {
                (
                    o.index,
                    match_score(o.node, current),
                    o.node.entity_id.clone(),
                )
            })
            .collect();
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        let Some((best_idx, best_score, best_id)) = candidates.first().cloned() else {
            continue;
        };
        if best_score < MATCH_THRESHOLD {
            continue;
        }
        if let Some((_, second_score, _)) = candidates.get(1) {
            if (best_score - second_score) < MATCH_TIE_DELTA {
                continue;
            }
        }
        assignments.insert(idx, best_id);
        used_old.insert(best_idx);
    }

    assignments
}

fn match_score(old: &FileEntityNode, new: &FlatNode) -> f64 {
    TITLE_WEIGHT * similarity(&old.title, &new.title)
        + PARENT_WEIGHT * similarity_opt(old.parent_title.as_deref(), new.parent_title.as_deref())
        + DEPTH_WEIGHT * depth_similarity(old.depth, new.depth)
        + POSITION_WEIGHT * position_similarity(old.position, new.position)
}

fn depth_similarity(a: usize, b: usize) -> f64 {
    1.0 / (1.0 + a.abs_diff(b) as f64)
}

fn position_similarity(a: usize, b: usize) -> f64 {
    1.0 / (1.0 + a.abs_diff(b) as f64)
}

fn similarity_opt(a: Option<&str>, b: Option<&str>) -> f64 {
    match (a, b) {
        (None, None) => 1.0,
        (Some(x), Some(y)) => similarity(x, y),
        _ => 0.0,
    }
}

fn similarity(a: &str, b: &str) -> f64 {
    let a_norm = normalize(a);
    let b_norm = normalize(b);
    if a_norm.is_empty() && b_norm.is_empty() {
        return 1.0;
    }
    if a_norm == b_norm {
        return 1.0;
    }
    let a_tokens: HashSet<&str> = a_norm.split_whitespace().collect();
    let b_tokens: HashSet<&str> = b_norm.split_whitespace().collect();
    if a_tokens.is_empty() || b_tokens.is_empty() {
        return 0.0;
    }
    let inter = a_tokens.intersection(&b_tokens).count() as f64;
    let union = a_tokens.union(&b_tokens).count() as f64;
    inter / union
}

fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c.is_ascii_whitespace() {
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn load_items_at_ref(root: &Path, commit_sha: &str, path_key: &str) -> Vec<FileItem> {
    let path = PathBuf::from(path_key);
    let Some(content) = git::file_content_at_ref(root, commit_sha, &path) else {
        return Vec::new();
    };
    parser::parse(&content, path)
}

fn flatten_nodes(items: &[FileItem]) -> Vec<FlatNode> {
    let mut raw = Vec::new();
    for item in items {
        let FileItem::Task(task) = item else {
            continue;
        };
        raw.push((
            vec![task.title.clone()],
            task.status.clone(),
            task.title.clone(),
            1usize,
            None,
        ));
        flatten_subtasks(
            &mut raw,
            &[task.title.clone()],
            &task.children,
            2,
            Some(task.title.clone()),
        );
    }
    let mut occurrence_index: HashMap<Vec<String>, usize> = HashMap::new();
    raw.into_iter()
        .enumerate()
        .map(|(position, (path, status, title, depth, parent_title))| {
            let occurrence = occurrence_index.entry(path.clone()).or_insert(0);
            let key = TransitionKey {
                path: path.clone(),
                occurrence: *occurrence,
            };
            *occurrence += 1;
            FlatNode {
                key,
                status,
                title,
                depth,
                parent_title,
                position,
            }
        })
        .collect()
}

fn flatten_subtasks(
    out: &mut Vec<(Vec<String>, Status, String, usize, Option<String>)>,
    parent_path: &[String],
    children: &[parser::Subtask],
    depth: usize,
    parent_title: Option<String>,
) {
    for child in children {
        let mut path = parent_path.to_vec();
        path.push(child.title.clone());
        out.push((
            path.clone(),
            child.status.clone(),
            child.title.clone(),
            depth,
            parent_title.clone(),
        ));
        flatten_subtasks(
            out,
            &path,
            &child.children,
            depth + 1,
            Some(child.title.clone()),
        );
    }
}

fn apply_transitions(
    entities: &mut HashMap<String, CachedEntity>,
    commit_sha: &str,
    commit_date: &str,
    transitions: &[EntityTransition],
) {
    for t in transitions {
        let Some(entity) = entities.get_mut(&t.entity_id) else {
            continue;
        };
        entity.last_seen_commit = commit_sha.to_string();
        entity.latest_status = t.new_status;
        if let Some(old) = t.old_status {
            if !is_cached_closed(old) && is_cached_closed(t.new_status) {
                entity.latest_close_date = Some(commit_date.to_string());
            }
        }
    }
}

fn is_closed(status: &Status) -> bool {
    matches!(status, Status::Done | Status::Cancelled)
}

fn is_cached_closed(status: CachedStatus) -> bool {
    matches!(status, CachedStatus::Done | CachedStatus::Cancelled)
}

fn cached_status_from(status: &Status) -> CachedStatus {
    match status {
        Status::Todo => CachedStatus::Todo,
        Status::Done => CachedStatus::Done,
        Status::Cancelled => CachedStatus::Cancelled,
    }
}

fn new_entity_id(next: &mut u64) -> String {
    let out = format!("e{:08}", *next);
    *next += 1;
    out
}

fn cache_file_path(root: &Path) -> Option<PathBuf> {
    Some(
        git::git_dir(root)?
            .join("mdagile")
            .join("lifecycle-cache.json"),
    )
}

fn read_cache_file(path: &Path) -> Option<LifecycleCache> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_cache_file(path: &Path, cache: &LifecycleCache) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(cache)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, format!("{payload}\n"))
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn normalize_repo_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        if let Ok(stripped) = path.strip_prefix(root) {
            return stripped.to_path_buf();
        }
        return path.to_path_buf();
    }
    let raw = path.to_string_lossy();
    if let Some(stripped) = raw.strip_prefix("./") {
        return PathBuf::from(stripped);
    }
    path.to_path_buf()
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
