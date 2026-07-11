use super::*;
use crate::config::{Config, GroupConfig, UserConfig};
use crate::parser::{FileItem, parse};
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
                        emails: vec![],
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "alice");
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "alice");
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "mallory");
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "bob");
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "mallory");
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "mallory");
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "mallory");
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "mallory");
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "mallory");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 2);
}

#[test]
fn no_head_version_new_task_already_done_and_misassigned_is_flagged() {
    let new = "\
- [x] brand new task @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(None, &p(new), &config, "mallory");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UnauthorizedCompletion);
}

#[test]
fn no_head_version_new_task_already_done_and_authorized_is_not_flagged() {
    let new = "\
- [x] brand new task @alice
";
    let config = config_with_users_and_groups(&["alice"], &[]);
    let issues = unauthorized_completion(None, &p(new), &config, "alice");
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "mallory");
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
    let issues = unauthorized_completion(Some(&p(old)), &p(new), &config, "mallory");
    assert_eq!(
        issues[0].data,
        Some(IssueData::UnauthorizedCompletion {
            authorized: vec!["alice".to_string()],
        })
    );
}
