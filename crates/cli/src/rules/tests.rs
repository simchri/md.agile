use super::*;
use crate::config::{Config, UserConfig};
use crate::parser::parse;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

fn config_with_users(users: &[&str]) -> Config {
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
        ..Config::default()
    }
}

fn first_task(items: &[FileItem]) -> &Task {
    match &items[0] {
        FileItem::Task(t) => t,
        _ => panic!("expected a task"),
    }
}

#[test]
fn wrong_indentation_vs_orphan_distinction_1() {
    // An Orphan task is:
    // - task that is separated by an empty line from previous elements
    // - AND has any indentation other than zero.
    //
    // Wrong indentation
    // - a sub task / task that is attached to a parent task (no empty line between other element)
    // - AND that has an indentation that does not match the subtask level

    let input = "\
- [ ] testing
  - [ ] subtask -> OK
  - [ ] another subtask -> OK
   - [ ] subtask WRONG INDENT
 - [ ] subtask WRONG INDENT

 - [ ] ORPHAN (also incorrect indent, but message should be orphan)

  - [ ] ORPHAN
";

    let mut issues = check_all(&p(input), &crate::config::Config::default());
    issues.sort_by_key(|i| i.location.line);

    assert_eq!(issues[0].code, ErrorCode::WrongIndentation);
    assert_eq!(issues[1].code, ErrorCode::WrongIndentation);
    assert_eq!(issues[2].code, ErrorCode::OrphanedSubtask);
    assert_eq!(issues[3].code, ErrorCode::OrphanedSubtask);
}

#[test]
fn missing_space_behind_box_vs_wrong_body_indent() {
    let input = "\
- [ ] ok, has space
- [ ]MISSING space
  - [ ] valid subtask
WRONG INDENT tasks description
";

    let mut issues = check_all(&p(input), &crate::config::Config::default());
    issues.sort_by_key(|i| i.location.line);

    assert_eq!(issues[0].code, ErrorCode::MissingSpaceAfterBox);
    assert_eq!(issues[0].location.line, 2);

    assert_eq!(issues[1].code, ErrorCode::WrongBodyIndentation);
    assert_eq!(issues[1].location.line, 4);
}

#[test]
fn check_config_independent_skips_undefined_property_and_assignment() {
    // #undeclared/@undeclared would be flagged by check_all against any
    // config that doesn't declare them (including Config::default()), but
    // check_config_independent must never look at properties/users/groups
    // at all — it's meant for use when there's no config worth trusting.
    let input = "\
- [ ] task #undeclared @undeclared
";
    let issues = check_config_independent(&p(input));
    assert!(
        !issues
            .iter()
            .any(|i| i.code == ErrorCode::UndefinedProperty),
        "check_config_independent must not run undefined_property, got: {issues:?}"
    );
    assert!(
        !issues
            .iter()
            .any(|i| i.code == ErrorCode::UndefinedAssignment),
        "check_config_independent must not run undefined_assignment, got: {issues:?}"
    );
}

#[test]
fn check_config_independent_still_runs_structural_checks() {
    // Structural checks (that don't need config) must still run.
    let input = "\
- [ ] 
";
    let issues = check_config_independent(&p(input));
    assert!(
        issues.iter().any(|i| i.code == ErrorCode::EmptyTitle),
        "check_config_independent should still run structural checks like empty_title, got: {issues:?}"
    );
}

#[test]
fn check_all_equals_check_config_independent_plus_config_dependent_checks() {
    // check_all must be exactly the union of check_config_independent and
    // the four config-dependent rules — no rule silently duplicated or
    // dropped when the two were split apart.
    let input = "\
- [ ] task #undeclared @undeclared
";
    let config = crate::config::Config::default();
    let mut from_check_all = check_all(&p(input), &config);
    let mut from_independent_plus_config_dependent = check_config_independent(&p(input));
    from_independent_plus_config_dependent.extend(undefined_property(&p(input), &config));
    from_independent_plus_config_dependent.extend(undefined_assignment(&p(input), &config));
    from_independent_plus_config_dependent.extend(missing_required_subtasks(&p(input), &config));
    from_independent_plus_config_dependent.extend(unrequired_quoted_subtask(&p(input), &config));

    from_check_all.sort_by_key(|i| (i.location.line, format!("{:?}", i.code)));
    from_independent_plus_config_dependent
        .sort_by_key(|i| (i.location.line, format!("{:?}", i.code)));

    assert_eq!(from_check_all, from_independent_plus_config_dependent);
}

#[test]
fn find_node_by_line_locates_top_level_task() {
    let input = "\
- [ ] first task
  - [ ] a subtask
- [ ] second task
";
    let items = p(input);
    let node = find_node_by_line(&items, 3).expect("expected a node at line 3");
    assert_eq!(node.title(), "second task");
}

#[test]
fn find_node_by_line_locates_nested_subtask() {
    let input = "\
- [ ] first task
  - [ ] a subtask
    - [ ] a nested subtask
";
    let items = p(input);
    let node = find_node_by_line(&items, 3).expect("expected a node at line 3");
    assert_eq!(node.title(), "a nested subtask");
}

#[test]
fn find_node_by_line_returns_none_when_no_node_starts_there() {
    let input = "\
- [ ] only task
";
    let items = p(input);
    assert!(find_node_by_line(&items, 2).is_none());
    assert!(find_node_by_line(&items, 0).is_none());
}

// ── is_eligible_for: recursive leaf-based eligibility ─────────────────────────

#[test]
fn unassigned_leaf_task_is_eligible_for_anyone() {
    let input = "\
- [ ] a leaf task
";
    let items = p(input);
    let task = first_task(&items);
    let config = config_with_users(&["alice"]);
    assert!(is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("alice".to_string()),
        &config
    ));
}

#[test]
fn task_assigned_to_someone_else_is_not_eligible() {
    let input = "\
- [ ] a leaf task @bob
";
    let items = p(input);
    let task = first_task(&items);
    let config = config_with_users(&["alice", "bob"]);
    assert!(!is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("alice".to_string()),
        &config
    ));
}

#[test]
fn unassigned_parent_is_ineligible_when_every_leaf_is_assigned_elsewhere() {
    // The parent itself carries no `@` marker, but every actionable leaf
    // subtask is assigned to someone else - there is nothing for `alice`
    // to do here, so the parent as a whole must not be considered eligible.
    let input = "\
- [ ] some #feature
  - [ ] first subtask @bob
  - [ ] second subtask @bob
";
    let items = p(input);
    let task = first_task(&items);
    let config = config_with_users(&["alice", "bob"]);
    assert!(!is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("alice".to_string()),
        &config
    ));
}

#[test]
fn unassigned_parent_is_eligible_when_at_least_one_leaf_is_eligible() {
    let input = "\
- [ ] some #feature
  - [ ] first subtask @bob
  - [ ] second subtask
";
    let items = p(input);
    let task = first_task(&items);
    let config = config_with_users(&["alice", "bob"]);
    assert!(is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("alice".to_string()),
        &config
    ));
}

#[test]
fn done_and_cancelled_leaves_assigned_to_me_do_not_count_as_eligible_leaves() {
    // Both leaves are technically assigned to `alice`, but neither is
    // actionable any more (one done, one cancelled) - there's nothing left
    // to do, so the parent must not be considered eligible.
    let input = "\
- [ ] some #feature
  - [x] first subtask @alice
  - [-] second subtask @alice
";
    let items = p(input);
    let task = first_task(&items);
    let config = config_with_users(&["alice"]);
    assert!(!is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("alice".to_string()),
        &config
    ));
}

#[test]
fn optional_leaf_still_counts_toward_eligibility() {
    let input = "\
- [ ] some #feature
  - [ ] first subtask @bob
  - [ ] #OPT optional subtask
";
    let items = p(input);
    let task = first_task(&items);
    let config = config_with_users(&["alice", "bob"]);
    assert!(is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("alice".to_string()),
        &config
    ));
}

#[test]
fn deeply_nested_eligible_leaf_makes_grandparent_eligible() {
    let input = "\
- [ ] some #feature
  - [ ] mid level @bob
    - [ ] deep leaf
";
    let items = p(input);
    let task = first_task(&items);
    let config = config_with_users(&["alice", "bob"]);
    assert!(is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("alice".to_string()),
        &config
    ));
}
