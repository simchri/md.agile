//! Per-entity lifecycle cache used by `agile history`.
//!
//! Tracks every task/subtask and every milestone across commit history under a
//! stable synthetic ID, recording an append-only list of transitions per
//! entity: status changes (done/cancelled/reopened), deletions, and — for
//! top-level tasks and milestones — rank changes. There is no separate
//! per-commit aggregate cache; anything else needed (e.g. velocity, plots)
//! must be recomputed from this transition log.

use crate::eta::{TodoDonePlotPoint, TransitionKey};
use crate::git;
use crate::parser::{self, FileItem, Status};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CACHE_VERSION: u32 = 3;
const MATCH_THRESHOLD: f64 = 0.70;
const MATCH_TIE_DELTA: f64 = 0.05;
const TITLE_WEIGHT: f64 = 0.50;
const PARENT_WEIGHT: f64 = 0.25;
const DEPTH_WEIGHT: f64 = 0.15;
const POSITION_WEIGHT: f64 = 0.10;
const MILESTONE_NAME_WEIGHT: f64 = 0.85;
const MILESTONE_POSITION_WEIGHT: f64 = 0.15;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LifecycleCache {
    pub version: u32,
    pub head_commit: String,
    pub commit_chain: Vec<String>, // oldest -> newest
    pub entities: HashMap<String, CachedEntity>,
    pub milestones: HashMap<String, CachedMilestone>,
    pub files: HashMap<String, FileHeadState>,
    pub milestone_files: HashMap<String, MilestoneHeadState>,
    pub next_entity_id: u64,
    pub next_milestone_id: u64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CachedEntity {
    pub first_seen_commit: String,
    pub last_seen_commit: String,
    pub fingerprint: Fingerprint,
    pub last_known_status: CachedStatus,
    /// Position among top-level tasks in the global (cross-file) priority
    /// order, 1-based. `None` for subtasks, which have no independent rank.
    pub last_known_rank: Option<usize>,
    /// For subtasks, the entity ID of their nearest top-level task ancestor
    /// (used to look up an inherited rank for scoping plots). `None` for
    /// top-level tasks themselves. Reflects the *current* ancestor only —
    /// re-parenting across commits isn't tracked historically.
    pub top_level_entity_id: Option<String>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transition {
    pub commit_hash: String,
    pub date: String,
    pub kind: TransitionKind,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TransitionKind {
    Done,
    Cancelled,
    /// A closed (done/cancelled) task moved back to todo.
    Reopened,
    /// The task disappeared from its file entirely.
    Deleted,
    /// Only ever recorded for top-level tasks.
    RankChanged {
        old_rank: usize,
        new_rank: usize,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fingerprint {
    pub title: String,
    pub depth: usize,
    pub parent_title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CachedStatus {
    Todo,
    Done,
    Cancelled,
}

/// Per-file matching/bookkeeping state for tasks — not the source of truth
/// for entity data (that lives in `entities`), just what's needed to match
/// live nodes across commits, plus a graveyard of deleted-but-reusable nodes.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileHeadState {
    #[serde(default)]
    pub entities: Vec<FileEntityNode>,
    #[serde(default)]
    pub graveyard: Vec<FileEntityNode>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileEntityNode {
    pub entity_id: String,
    pub key: KeyRepr,
    pub title: String,
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
pub struct CachedMilestone {
    pub first_seen_commit: String,
    pub last_seen_commit: String,
    pub name: String,
    /// The rank of the task immediately preceding this milestone in the
    /// global priority order. `None` if no task precedes it.
    pub last_known_rank: Option<usize>,
    pub transitions: Vec<MilestoneTransition>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MilestoneTransition {
    pub commit_hash: String,
    pub date: String,
    pub kind: MilestoneTransitionKind,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MilestoneTransitionKind {
    RankChanged {
        old_rank: Option<usize>,
        new_rank: Option<usize>,
    },
    Deleted,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MilestoneHeadState {
    #[serde(default)]
    pub milestones: Vec<FileMilestoneNode>,
    #[serde(default)]
    pub graveyard: Vec<FileMilestoneNode>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileMilestoneNode {
    pub milestone_id: String,
    pub key: MilestoneKeyRepr,
    pub name: String,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MilestoneKeyRepr {
    pub name: String,
    pub occurrence: usize,
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

#[derive(Debug, Clone)]
struct FlatMilestone {
    key: MilestoneKeyRepr,
    name: String,
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

/// Returns, for every currently-closed node in `current_items`, the date of
/// its most recent close (done/cancelled) transition — or omits it if that
/// can't be determined from committed history (e.g. the close is uncommitted).
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
        .chain(file_state.graveyard.iter())
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
        let Some(date) = latest_close_date(entity) else {
            continue;
        };
        out.insert(current.key.clone(), date.to_string());
    }
    out
}

fn latest_close_date(entity: &CachedEntity) -> Option<&str> {
    for t in entity.transitions.iter().rev() {
        match &t.kind {
            TransitionKind::Done | TransitionKind::Cancelled => return Some(t.date.as_str()),
            TransitionKind::Reopened => return None,
            TransitionKind::Deleted | TransitionKind::RankChanged { .. } => continue,
        }
    }
    None
}

/// Per-entity replay state used to reconstruct historical scope/status
/// without re-parsing any commit content: everything needed is already in
/// the entity's transition log.
struct EntityTimeline<'a> {
    depth: usize,
    weight: f64,
    first_index: usize,
    /// Index of the commit at which this entity was deleted, if ever. The
    /// entity counts in neither total nor done from this commit onward.
    deleted_index: Option<usize>,
    top_level_entity_id: Option<&'a str>,
    /// (commit_index, rank) breakpoints, ascending by commit_index. Only
    /// populated for top-level tasks (depth == 1); rank holds constant
    /// between breakpoints.
    rank_breaks: Vec<(usize, usize)>,
    /// (commit_index, is_closed) breakpoints, ascending by commit_index.
    closed_breaks: Vec<(usize, bool)>,
}

/// Reads the step-function value in `breaks` that's in effect at `commit_index`
/// (the value from the latest breakpoint at or before `commit_index`).
fn step_value<T: Copy>(breaks: &[(usize, T)], commit_index: usize) -> Option<T> {
    breaks
        .iter()
        .rev()
        .find(|(idx, _)| *idx <= commit_index)
        .map(|(_, v)| *v)
}

/// Recomputes a "to-do vs done" time series scoped to a milestone, purely
/// from the already-built lifecycle cache — no re-parsing of historical
/// commit content is needed.
///
/// `target_rank` is the milestone's rank (the rank of the top-level task
/// immediately preceding it), treated as fixed across the whole timeline: we
/// don't replay the milestone's own historical rank changes, we just ask "was
/// this task's rank at or before the milestone's *current* rank at the time".
/// `None` means the milestone precedes every task, so nothing is ever in
/// scope.
///
/// Known simplification: if an entity's very first recorded status
/// transition is `Cancelled` (with no preceding `Done`), we can't tell
/// whether it started out open or already done — we assume it started open,
/// since a task being cancelled directly from todo is the far more common
/// case. Similarly, a subtask's top-level ancestor is resolved using its
/// *current* ancestor only; re-parenting across commits isn't tracked.
pub fn todo_done_timeline(
    cache: &LifecycleCache,
    commits: &[git::CommitRef],
    target_rank: Option<usize>,
) -> Vec<TodoDonePlotPoint> {
    let commit_index: HashMap<&str, usize> = cache
        .commit_chain
        .iter()
        .enumerate()
        .map(|(i, sha)| (sha.as_str(), i))
        .collect();
    let commit_dates: Vec<String> = cache
        .commit_chain
        .iter()
        .map(|sha| {
            commits
                .iter()
                .find(|c| &c.sha == sha)
                .map(|c| unix_to_yyyy_mm_dd(c.timestamp))
                .unwrap_or_default()
        })
        .collect();

    let mut timelines: HashMap<&str, EntityTimeline<'_>> = HashMap::new();
    for (id, entity) in &cache.entities {
        let Some(&first_index) = commit_index.get(entity.first_seen_commit.as_str()) else {
            continue;
        };
        let deleted_index = entity.transitions.iter().find_map(|t| match t.kind {
            TransitionKind::Deleted => commit_index.get(t.commit_hash.as_str()).copied(),
            _ => None,
        });

        let mut rank_breaks = Vec::new();
        if entity.fingerprint.depth == 1 {
            let initial_rank = entity
                .transitions
                .iter()
                .find_map(|t| match t.kind {
                    TransitionKind::RankChanged { old_rank, .. } => Some(old_rank),
                    _ => None,
                })
                .or(entity.last_known_rank)
                .unwrap_or(1);
            rank_breaks.push((first_index, initial_rank));
            for t in &entity.transitions {
                if let TransitionKind::RankChanged { new_rank, .. } = t.kind {
                    if let Some(&idx) = commit_index.get(t.commit_hash.as_str()) {
                        rank_breaks.push((idx, new_rank));
                    }
                }
            }
        }

        let initial_closed = entity
            .transitions
            .iter()
            .find_map(|t| match t.kind {
                TransitionKind::Done | TransitionKind::Cancelled => Some(false),
                TransitionKind::Reopened => Some(true),
                _ => None,
            })
            .unwrap_or_else(|| !matches!(entity.last_known_status, CachedStatus::Todo));
        let mut closed_breaks = vec![(first_index, initial_closed)];
        for t in &entity.transitions {
            let new_closed = match t.kind {
                TransitionKind::Done | TransitionKind::Cancelled => Some(true),
                TransitionKind::Reopened => Some(false),
                _ => None,
            };
            if let Some(closed) = new_closed {
                if let Some(&idx) = commit_index.get(t.commit_hash.as_str()) {
                    closed_breaks.push((idx, closed));
                }
            }
        }

        timelines.insert(
            id.as_str(),
            EntityTimeline {
                depth: entity.fingerprint.depth,
                weight: crate::eta::weight_for_depth(entity.fingerprint.depth),
                first_index,
                deleted_index,
                top_level_entity_id: entity.top_level_entity_id.as_deref(),
                rank_breaks,
                closed_breaks,
            },
        );
    }

    let is_alive_at = |t: &EntityTimeline<'_>, i: usize| {
        i >= t.first_index && t.deleted_index.map_or(true, |d| i < d)
    };

    (0..cache.commit_chain.len())
        .map(|i| {
            let mut total_weight = 0.0;
            let mut done_weight = 0.0;
            let mut total_count = 0;
            let mut done_count = 0;
            for timeline in timelines.values() {
                if !is_alive_at(timeline, i) {
                    continue;
                }
                let rank = if timeline.depth == 1 {
                    step_value(&timeline.rank_breaks, i)
                } else {
                    match timeline
                        .top_level_entity_id
                        .and_then(|tid| timelines.get(tid))
                    {
                        Some(ancestor) if is_alive_at(ancestor, i) => {
                            step_value(&ancestor.rank_breaks, i)
                        }
                        _ => None,
                    }
                };
                let Some(rank) = rank else { continue };
                let in_scope = target_rank.is_some_and(|r| rank <= r);
                if !in_scope {
                    continue;
                }
                total_weight += timeline.weight;
                total_count += 1;
                if step_value(&timeline.closed_breaks, i).unwrap_or(false) {
                    done_weight += timeline.weight;
                    done_count += 1;
                }
            }
            TodoDonePlotPoint {
                date: commit_dates[i].clone(),
                total_weight,
                done_weight,
                total_count,
                done_count,
            }
        })
        .collect()
}

fn append_new_commits(
    root: &Path,
    mut cache: LifecycleCache,
    commits: &[git::CommitRef],
) -> Option<LifecycleCache> {
    let mut prev_files = cache.files.clone();
    let mut prev_milestone_files = cache.milestone_files.clone();
    let mut prev_commit = cache.head_commit.clone();
    for commit in commits.iter().skip(cache.commit_chain.len()) {
        let date = unix_to_yyyy_mm_dd(commit.timestamp);
        let (new_files, new_milestone_files, next_entity_id, next_milestone_id) = advance_commit(
            root,
            &prev_files,
            &prev_milestone_files,
            &prev_commit,
            &commit.sha,
            &date,
            cache.next_entity_id,
            cache.next_milestone_id,
            &mut cache.entities,
            &mut cache.milestones,
        )?;
        cache.next_entity_id = next_entity_id;
        cache.next_milestone_id = next_milestone_id;
        cache.commit_chain.push(commit.sha.clone());
        cache.head_commit = commit.sha.clone();
        prev_commit = commit.sha.clone();
        prev_files = new_files;
        prev_milestone_files = new_milestone_files;
    }
    cache.files = prev_files;
    cache.milestone_files = prev_milestone_files;
    Some(cache)
}

fn rebuild_from_scratch(
    root: &Path,
    commits: &[git::CommitRef],
    head_commit: &str,
) -> Option<LifecycleCache> {
    let mut entities: HashMap<String, CachedEntity> = HashMap::new();
    let mut milestones: HashMap<String, CachedMilestone> = HashMap::new();
    let mut next_entity_id = 1u64;
    let mut next_milestone_id = 1u64;

    let first_commit = commits.first()?;
    let first_date = unix_to_yyyy_mm_dd(first_commit.timestamp);
    // Advancing from the first commit to itself with empty previous state
    // naturally produces the initial snapshot: every node mints a fresh ID
    // with no transitions (there's nothing to diff against yet).
    let (mut files, mut milestone_files, next_e, next_m) = advance_commit(
        root,
        &HashMap::new(),
        &HashMap::new(),
        &first_commit.sha,
        &first_commit.sha,
        &first_date,
        next_entity_id,
        next_milestone_id,
        &mut entities,
        &mut milestones,
    )?;
    next_entity_id = next_e;
    next_milestone_id = next_m;

    let mut prev_commit = first_commit.sha.clone();
    for commit in commits.iter().skip(1) {
        let date = unix_to_yyyy_mm_dd(commit.timestamp);
        let (new_files, new_milestone_files, next_e, next_m) = advance_commit(
            root,
            &files,
            &milestone_files,
            &prev_commit,
            &commit.sha,
            &date,
            next_entity_id,
            next_milestone_id,
            &mut entities,
            &mut milestones,
        )?;
        next_entity_id = next_e;
        next_milestone_id = next_m;
        files = new_files;
        milestone_files = new_milestone_files;
        prev_commit = commit.sha.clone();
    }

    Some(LifecycleCache {
        version: CACHE_VERSION,
        head_commit: head_commit.to_string(),
        commit_chain: commits.iter().map(|c| c.sha.clone()).collect(),
        entities,
        milestones,
        files,
        milestone_files,
        next_entity_id,
        next_milestone_id,
    })
}

struct GlobalLayout {
    // Rank (1-based, among top-level tasks) keyed by (file path, task key).
    task_ranks: HashMap<(String, TransitionKey), usize>,
    // Rank of the nearest preceding top-level task, keyed by (file path,
    // milestone key). `None` if no task precedes the milestone.
    milestone_ranks: HashMap<(String, MilestoneKeyRepr), Option<usize>>,
}

/// Computes the global (cross-file) priority-order rank of every top-level
/// task at `commit_sha`, plus — for every milestone — the rank of the
/// nearest preceding top-level task in that same global order.
fn compute_global_layout(root: &Path, commit_sha: &str) -> GlobalLayout {
    let mut paths = git::task_files_at_ref(root, commit_sha);
    paths.sort();

    let mut task_ranks = HashMap::new();
    let mut milestone_ranks = HashMap::new();
    let mut current_rank = 0usize;
    let mut last_rank: Option<usize> = None;

    for path in paths {
        let Some(content) = git::file_content_at_ref(root, commit_sha, &path) else {
            continue;
        };
        let items = parser::parse(&content, path.clone());
        let path_s = path_key(&path);
        let mut top_level_nodes = flatten_nodes(&items).into_iter().filter(|n| n.depth == 1);
        let mut milestone_occurrence: HashMap<String, usize> = HashMap::new();

        for item in &items {
            match item {
                FileItem::Task(_) => {
                    if let Some(node) = top_level_nodes.next() {
                        current_rank += 1;
                        task_ranks.insert((path_s.clone(), node.key.clone()), current_rank);
                        last_rank = Some(current_rank);
                    }
                }
                FileItem::Milestone(m) => {
                    let occ = milestone_occurrence.entry(m.name.clone()).or_insert(0);
                    let mkey = MilestoneKeyRepr {
                        name: m.name.clone(),
                        occurrence: *occ,
                    };
                    *occ += 1;
                    milestone_ranks.insert((path_s.clone(), mkey), last_rank);
                }
            }
        }
    }

    GlobalLayout {
        task_ranks,
        milestone_ranks,
    }
}

#[allow(clippy::too_many_arguments)]
fn advance_commit(
    root: &Path,
    previous_files: &HashMap<String, FileHeadState>,
    previous_milestone_files: &HashMap<String, MilestoneHeadState>,
    old_commit: &str,
    new_commit: &str,
    date: &str,
    mut next_entity_id: u64,
    mut next_milestone_id: u64,
    entities: &mut HashMap<String, CachedEntity>,
    milestones: &mut HashMap<String, CachedMilestone>,
) -> Option<(
    HashMap<String, FileHeadState>,
    HashMap<String, MilestoneHeadState>,
    u64,
    u64,
)> {
    let old_paths: HashSet<String> = git::task_files_at_ref(root, old_commit)
        .into_iter()
        .map(|p| path_key(&p))
        .collect();
    let new_paths: HashSet<String> = git::task_files_at_ref(root, new_commit)
        .into_iter()
        .map(|p| path_key(&p))
        .collect();
    let all_paths: HashSet<String> = old_paths.union(&new_paths).cloned().collect();

    let layout = compute_global_layout(root, new_commit);

    let mut new_files = HashMap::new();
    let mut new_milestone_files = HashMap::new();
    for path in all_paths {
        let prev_state = previous_files.get(&path).cloned().unwrap_or_default();
        let prev_milestone_state = previous_milestone_files
            .get(&path)
            .cloned()
            .unwrap_or_default();
        let new_items = load_items_at_ref(root, new_commit, &path);
        let flat_nodes = flatten_nodes(&new_items);
        let flat_milestones = flatten_milestones(&new_items);

        let state = assign_entities(
            &prev_state,
            &flat_nodes,
            new_commit,
            date,
            &path,
            &layout.task_ranks,
            &mut next_entity_id,
            entities,
        );
        let milestone_state = assign_milestones(
            &prev_milestone_state,
            &flat_milestones,
            new_commit,
            date,
            &path,
            &layout.milestone_ranks,
            &mut next_milestone_id,
            milestones,
        );

        if !state.entities.is_empty() || !state.graveyard.is_empty() {
            new_files.insert(path.clone(), state);
        }
        if !milestone_state.milestones.is_empty() || !milestone_state.graveyard.is_empty() {
            new_milestone_files.insert(path, milestone_state);
        }
    }
    Some((
        new_files,
        new_milestone_files,
        next_entity_id,
        next_milestone_id,
    ))
}

#[allow(clippy::too_many_arguments)]
fn assign_entities(
    prev_state: &FileHeadState,
    new_nodes: &[FlatNode],
    commit_sha: &str,
    date: &str,
    path: &str,
    task_ranks: &HashMap<(String, TransitionKey), usize>,
    next_entity_id: &mut u64,
    entities: &mut HashMap<String, CachedEntity>,
) -> FileHeadState {
    let pool: Vec<HeadNodeView<'_>> = prev_state
        .entities
        .iter()
        .chain(prev_state.graveyard.iter())
        .enumerate()
        .map(|(index, node)| HeadNodeView {
            index,
            node,
            used: false,
        })
        .collect();
    let assignments = assign_current_to_head(&pool, new_nodes);

    let mut out_live = Vec::with_capacity(new_nodes.len());
    let mut used_pool_indices: HashSet<usize> = HashSet::new();
    let mut current_top_level_id: Option<String> = None;
    for (idx, node) in new_nodes.iter().enumerate() {
        let entity_id = assignments
            .get(&idx)
            .cloned()
            .unwrap_or_else(|| new_entity_id(next_entity_id));
        if let Some(pool_idx) = pool
            .iter()
            .find(|v| v.node.entity_id == entity_id)
            .map(|v| v.index)
        {
            used_pool_indices.insert(pool_idx);
        }
        let new_rank = if node.depth == 1 {
            task_ranks
                .get(&(path.to_string(), node.key.clone()))
                .copied()
        } else {
            None
        };
        let top_level_entity_id = if node.depth == 1 {
            None
        } else {
            current_top_level_id.clone()
        };
        upsert_entity(
            entities,
            &entity_id,
            node,
            commit_sha,
            date,
            new_rank,
            top_level_entity_id,
        );
        if node.depth == 1 {
            current_top_level_id = Some(entity_id.clone());
        }
        out_live.push(FileEntityNode {
            entity_id,
            key: KeyRepr {
                path: node.key.path.clone(),
                occurrence: node.key.occurrence,
            },
            title: node.title.clone(),
            depth: node.depth,
            parent_title: node.parent_title.clone(),
            position: node.position,
        });
    }

    let mut out_graveyard = Vec::new();
    for view in &pool {
        if used_pool_indices.contains(&view.index) {
            continue;
        }
        let was_already_dead = prev_state
            .graveyard
            .iter()
            .any(|g| g.entity_id == view.node.entity_id);
        if !was_already_dead {
            if let Some(entity) = entities.get_mut(&view.node.entity_id) {
                entity.last_seen_commit = commit_sha.to_string();
                entity.transitions.push(Transition {
                    commit_hash: commit_sha.to_string(),
                    date: date.to_string(),
                    kind: TransitionKind::Deleted,
                });
            }
        }
        out_graveyard.push(view.node.clone());
    }

    FileHeadState {
        entities: out_live,
        graveyard: out_graveyard,
    }
}

fn upsert_entity(
    entities: &mut HashMap<String, CachedEntity>,
    entity_id: &str,
    node: &FlatNode,
    commit_sha: &str,
    date: &str,
    new_rank: Option<usize>,
    top_level_entity_id: Option<String>,
) {
    let new_status = cached_status_from(&node.status);
    let fingerprint = Fingerprint {
        title: node.title.clone(),
        depth: node.depth,
        parent_title: node.parent_title.clone(),
    };
    match entities.get_mut(entity_id) {
        Some(entity) => {
            if entity.last_known_status != new_status {
                let kind = match new_status {
                    CachedStatus::Done => TransitionKind::Done,
                    CachedStatus::Cancelled => TransitionKind::Cancelled,
                    CachedStatus::Todo => TransitionKind::Reopened,
                };
                entity.transitions.push(Transition {
                    commit_hash: commit_sha.to_string(),
                    date: date.to_string(),
                    kind,
                });
                entity.last_known_status = new_status;
            }
            if let Some(new_r) = new_rank {
                if let Some(old_r) = entity.last_known_rank {
                    if old_r != new_r {
                        entity.transitions.push(Transition {
                            commit_hash: commit_sha.to_string(),
                            date: date.to_string(),
                            kind: TransitionKind::RankChanged {
                                old_rank: old_r,
                                new_rank: new_r,
                            },
                        });
                    }
                }
                entity.last_known_rank = Some(new_r);
            }
            entity.last_seen_commit = commit_sha.to_string();
            entity.fingerprint = fingerprint;
            entity.top_level_entity_id = top_level_entity_id;
        }
        None => {
            entities.insert(
                entity_id.to_string(),
                CachedEntity {
                    first_seen_commit: commit_sha.to_string(),
                    last_seen_commit: commit_sha.to_string(),
                    fingerprint,
                    last_known_status: new_status,
                    last_known_rank: new_rank,
                    top_level_entity_id,
                    transitions: Vec::new(),
                },
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn assign_milestones(
    prev_state: &MilestoneHeadState,
    new_milestones: &[FlatMilestone],
    commit_sha: &str,
    date: &str,
    path: &str,
    milestone_ranks: &HashMap<(String, MilestoneKeyRepr), Option<usize>>,
    next_milestone_id: &mut u64,
    milestones: &mut HashMap<String, CachedMilestone>,
) -> MilestoneHeadState {
    let pool: Vec<MilestoneHeadView<'_>> = prev_state
        .milestones
        .iter()
        .chain(prev_state.graveyard.iter())
        .enumerate()
        .map(|(index, node)| MilestoneHeadView {
            index,
            node,
            used: false,
        })
        .collect();
    let assignments = assign_milestones_to_new(&pool, new_milestones);

    let mut out_live = Vec::with_capacity(new_milestones.len());
    let mut used_pool_indices: HashSet<usize> = HashSet::new();
    for (idx, fm) in new_milestones.iter().enumerate() {
        let milestone_id = assignments
            .get(&idx)
            .cloned()
            .unwrap_or_else(|| new_milestone_id_str(next_milestone_id));
        if let Some(pool_idx) = pool
            .iter()
            .find(|v| v.node.milestone_id == milestone_id)
            .map(|v| v.index)
        {
            used_pool_indices.insert(pool_idx);
        }
        let new_rank = milestone_ranks
            .get(&(path.to_string(), fm.key.clone()))
            .cloned()
            .flatten();
        upsert_milestone(milestones, &milestone_id, fm, commit_sha, date, new_rank);
        out_live.push(FileMilestoneNode {
            milestone_id,
            key: fm.key.clone(),
            name: fm.name.clone(),
            position: fm.position,
        });
    }

    let mut out_graveyard = Vec::new();
    for view in &pool {
        if used_pool_indices.contains(&view.index) {
            continue;
        }
        let was_already_dead = prev_state
            .graveyard
            .iter()
            .any(|g| g.milestone_id == view.node.milestone_id);
        if !was_already_dead {
            if let Some(m) = milestones.get_mut(&view.node.milestone_id) {
                m.last_seen_commit = commit_sha.to_string();
                m.transitions.push(MilestoneTransition {
                    commit_hash: commit_sha.to_string(),
                    date: date.to_string(),
                    kind: MilestoneTransitionKind::Deleted,
                });
            }
        }
        out_graveyard.push(view.node.clone());
    }

    MilestoneHeadState {
        milestones: out_live,
        graveyard: out_graveyard,
    }
}

fn upsert_milestone(
    milestones: &mut HashMap<String, CachedMilestone>,
    milestone_id: &str,
    fm: &FlatMilestone,
    commit_sha: &str,
    date: &str,
    new_rank: Option<usize>,
) {
    match milestones.get_mut(milestone_id) {
        Some(m) => {
            if m.last_known_rank != new_rank {
                m.transitions.push(MilestoneTransition {
                    commit_hash: commit_sha.to_string(),
                    date: date.to_string(),
                    kind: MilestoneTransitionKind::RankChanged {
                        old_rank: m.last_known_rank,
                        new_rank,
                    },
                });
                m.last_known_rank = new_rank;
            }
            m.last_seen_commit = commit_sha.to_string();
            m.name = fm.name.clone();
        }
        None => {
            milestones.insert(
                milestone_id.to_string(),
                CachedMilestone {
                    first_seen_commit: commit_sha.to_string(),
                    last_seen_commit: commit_sha.to_string(),
                    name: fm.name.clone(),
                    last_known_rank: new_rank,
                    transitions: Vec::new(),
                },
            );
        }
    }
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

struct MilestoneHeadView<'a> {
    index: usize,
    node: &'a FileMilestoneNode,
    used: bool,
}

fn assign_milestones_to_new(
    old_nodes: &[MilestoneHeadView<'_>],
    new_nodes: &[FlatMilestone],
) -> HashMap<usize, String> {
    let mut assignments = HashMap::new();
    let mut used_old = HashSet::<usize>::new();

    let old_by_key: HashMap<MilestoneKeyRepr, &FileMilestoneNode> = old_nodes
        .iter()
        .map(|v| (v.node.key.clone(), v.node))
        .collect();
    for (idx, current) in new_nodes.iter().enumerate() {
        if let Some(node) = old_by_key.get(&current.key) {
            assignments.insert(idx, node.milestone_id.clone());
            if let Some(v) = old_nodes
                .iter()
                .find(|v| v.node.milestone_id == node.milestone_id)
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
                    milestone_match_score(o.node, current),
                    o.node.milestone_id.clone(),
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

fn milestone_match_score(old: &FileMilestoneNode, new: &FlatMilestone) -> f64 {
    MILESTONE_NAME_WEIGHT * similarity(&old.name, &new.name)
        + MILESTONE_POSITION_WEIGHT * position_similarity(old.position, new.position)
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

fn flatten_milestones(items: &[FileItem]) -> Vec<FlatMilestone> {
    let mut occurrence_index: HashMap<String, usize> = HashMap::new();
    let mut out = Vec::new();
    let mut position = 0usize;
    for item in items {
        let FileItem::Milestone(m) = item else {
            continue;
        };
        let occurrence = occurrence_index.entry(m.name.clone()).or_insert(0);
        let key = MilestoneKeyRepr {
            name: m.name.clone(),
            occurrence: *occurrence,
        };
        *occurrence += 1;
        out.push(FlatMilestone {
            key,
            name: m.name.clone(),
            position,
        });
        position += 1;
    }
    out
}

fn is_closed(status: &Status) -> bool {
    matches!(status, Status::Done | Status::Cancelled)
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

fn new_milestone_id_str(next: &mut u64) -> String {
    let out = format!("m{:08}", *next);
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
