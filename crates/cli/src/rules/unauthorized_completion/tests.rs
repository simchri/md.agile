use super::*;
use crate::config::{Config, GroupConfig, UserConfig};
use crate::parser::{FileItem, parse};
use crate::rules::ResolvedIdentity;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

fn config_with_users_and_groups(users: &[&str], groups: &[(&str, &[&str])]) -> Config {
    Config {
        users: users
            .iter()
            .map(|&n| {
                (
                    n.to_string(),
                    UserConfig {
                        name: n.to_string(),
                        git_emails: vec![],
                        git_names: vec![],
                    },
                )
            })
            .collect(),
        groups: groups
            .iter()
            .map(|&(n, members)| {
                (
                    n.to_string(),
                    GroupConfig {
                        name: n.to_string(),
                        members: members.iter().map(|s| s.to_string()).collect(),
                    },
                )
            })
            .collect(),
        ..Config::default()
    }
}

// ── no-issue cases ────────────────────────────────────────────────────────────

#[test]
fn authorized_direct_assignee_completes_task_no_issue() {
    let old = "\
- [ ] fix bug @alice
";
    let new = "\
- [x] fix bug @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("alice".to_string()),
    );
    assert!(issues.is_empty());
}

#[test]
fn group_assigned_task_completed_by_member_no_issue() {
    let old = "\
- [ ] fix bug @devs
";
    let new = "\
- [x] fix bug @devs
";
    let config = config_with_users_and_groups(&["alice", "bob"], &[("devs", &["alice", "bob"])]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("alice".to_string()),
    );
    assert!(issues.is_empty());
}

#[test]
fn unassigned_task_completed_by_anyone_no_issue() {
    let old = "\
- [ ] fix bug
";
    let new = "\
- [x] fix bug
";
    let config = Config::default();
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("mallory".to_string()),
    );
    assert!(issues.is_empty());
}

#[test]
fn multiple_assignees_one_match_no_issue() {
    let old = "\
- [ ] fix bug @alice @bob
";
    let new = "\
- [x] fix bug @alice @bob
";
    let config = config_with_users_and_groups(&["alice", "bob"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("bob".to_string()),
    );
    assert!(issues.is_empty());
}

#[test]
fn already_done_task_unchanged_is_not_rechecked() {
    // Task was already done in HEAD; re-parsing the (unchanged) working copy
    // must not re-trigger the check even though "mallory" isn't assigned.
    let old = "\
- [x] fix bug @alice
";
    let new = "\
- [x] fix bug @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("mallory".to_string()),
    );
    assert!(issues.is_empty());
}

#[test]
fn todo_task_remaining_todo_is_not_flagged() {
    let old = "\
- [ ] fix bug @alice
";
    let new = "\
- [ ] fix bug @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("mallory".to_string()),
    );
    assert!(issues.is_empty());
}

// ── error cases ───────────────────────────────────────────────────────────────

#[test]
fn unauthorized_user_completes_directly_assigned_task_is_flagged() {
    let old = "\
- [ ] fix bug @alice
";
    let new = "\
- [x] fix bug @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("mallory".to_string()),
    );
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UnauthorizedCompletion);
    assert_eq!(issues[0].location.line, 1);
    assert!(issues[0].message.contains("alice"), "{}", issues[0].message);
}

#[test]
fn group_assigned_task_completed_by_non_member_is_flagged() {
    let old = "\
- [ ] fix bug @devs
";
    let new = "\
- [x] fix bug @devs
";
    let config = config_with_users_and_groups(&["alice", "bob"], &[("devs", &["alice", "bob"])]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("mallory".to_string()),
    );
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UnauthorizedCompletion);
}

#[test]
fn nested_subtask_transition_is_checked() {
    let old = "\
- [ ] parent
  - [ ] child @alice
";
    let new = "\
- [ ] parent
  - [x] child @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("mallory".to_string()),
    );
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 2);
}

#[test]
fn no_head_version_new_task_already_done_and_misassigned_is_flagged() {
    let new = "\
- [x] brand new task @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        None,
        &p(new),
        &config,
        &ResolvedIdentity::Known("mallory".to_string()),
    );
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UnauthorizedCompletion);
}

#[test]
fn no_head_version_new_task_already_done_and_authorized_is_not_flagged() {
    let new = "\
- [x] brand new task @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        None,
        &p(new),
        &config,
        &ResolvedIdentity::Known("alice".to_string()),
    );
    assert!(issues.is_empty());
}

#[test]
fn title_changed_alongside_status_change_is_still_flagged() {
    // No matching title in the old version — treated as a new/transitioned task.
    let old = "\
- [ ] original title @alice
";
    let new = "\
- [x] renamed title @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("mallory".to_string()),
    );
    assert_eq!(issues.len(), 1);
}

#[test]
fn issue_data_includes_authorized_names() {
    let old = "\
- [ ] fix bug @alice
";
    let new = "\
- [x] fix bug @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("mallory".to_string()),
    );
    assert_eq!(
        issues[0].data,
        Some(IssueData::UnauthorizedCompletion {
            authorized: vec!["alice".to_string()],
        })
    );
}

// ── unrecognized identity ───────────────────────────────────────────────────────

#[test]
fn unrecognized_identity_completing_assigned_task_is_flagged() {
    let old = "\
- [ ] fix bug @alice
";
    let new = "\
- [x] fix bug @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Unrecognized,
    );
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UnauthorizedCompletion);
}

#[test]
fn unrecognized_identity_completing_unassigned_task_is_not_flagged() {
    let old = "\
- [ ] fix bug
";
    let new = "\
- [x] fix bug
";
    let config = Config::default();
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Unrecognized,
    );
    assert!(issues.is_empty());
}

// ── duplicate-titled node matching ──────────────────────────────────────────

#[test]
fn duplicate_subtask_titles_under_different_parents_are_matched_independently() {
    // Subtasks named "bar" under two different parent tasks (this is a very
    // common, expected pattern for #property-required subtasks, which by
    // design reuse the same literal title across every task carrying that
    // property — see mdagile.toml's own `subtasks = ["bar", "baz"]`).
    //
    // Task A's "bar" is a genuine new (unauthorized) transition. Task B's
    // "bar" was already done and is unchanged. Matching by bare title alone
    // (ignoring which parent task a subtask belongs to) can misattribute
    // task A's old status to task B's (or vice versa), since both share the
    // key "bar" in a flat, file-wide title lookup.
    let old = "\
- [ ] task A @alice
  - [ ] bar @alice
- [x] task B @alice
  - [x] bar @alice
";
    let new = "\
- [ ] task A @alice
  - [x] bar @alice
- [x] task B @alice
  - [x] bar @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Unrecognized,
    );
    assert_eq!(
        issues.len(),
        1,
        "expected task A's genuine bar transition to be flagged, got: {issues:?}"
    );
    assert_eq!(issues[0].location.line, 2, "should point at task A's 'bar'");
}
#[test]
fn duplicate_sibling_titles_are_matched_positionally_not_collapsed() {
    // Two siblings sharing the exact same title (and thus the same
    // ancestor-title path) is legal -- nothing forbids it. Only the
    // second one actually transitions to done; the first was already
    // done in the old (HEAD) version and must not be re-flagged.
    let old = "\
- [x] setup env @alice
- [ ] setup env @alice
";
    let new = "\
- [x] setup env @alice
- [x] setup env @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(
        Some(&p(old)),
        &p(new),
        &config,
        &ResolvedIdentity::Known("bob".to_string()),
    );
    assert_eq!(
        issues.len(),
        1,
        "only the genuinely-transitioned sibling should be flagged, got: {issues:?}"
    );
    assert_eq!(
        issues[0].location.line, 2,
        "should point at the second 'setup env' line, not the already-done first one"
    );
}
