use super::*;
use crate::config::{Config, UserConfig};
use crate::parser::{FileItem, Task, parse};
use crate::rules::NodeRef;
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

// ── is_eligible_for: recursive eligibility, generalized to all levels ──────

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
    // subtask is assigned to someone else - there is genuinely remaining
    // work here, just none of it actionable for `alice`, so the parent as a
    // whole must not be considered eligible.
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
fn done_and_cancelled_leaves_regardless_of_assignment_make_the_parent_itself_eligible() {
    // Both leaves are done/cancelled - nothing actionable remains under the
    // parent for *anyone*, regardless of who those now-resolved leaves used
    // to be assigned to. With nothing left to delegate to, the (unassigned)
    // parent itself becomes the eligible unit - this is the fix for the
    // reported bug where `agile task next --mine` skipped a task whose only
    // remaining status was "done at every leaf, but the parent itself still
    // marked todo": eligibility must not be restricted to literal leaves.
    let input = "\
- [ ] some #feature
  - [x] first subtask @alice
  - [-] second subtask @alice
";
    let items = p(input);
    let task = first_task(&items);
    let config = config_with_users(&["alice"]);
    assert!(is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("alice".to_string()),
        &config
    ));
    // ...and, being unassigned itself, for anyone else too.
    let config = config_with_users(&["alice", "bob"]);
    assert!(is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("bob".to_string()),
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
    // The mid-level task is unassigned here, so its own eligibility passes
    // through to its unassigned deep leaf, which in turn makes the whole
    // chain eligible for alice.
    let input = "\
- [ ] some #feature
  - [ ] mid level
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

#[test]
fn assignment_on_a_parent_claims_its_whole_subtree_even_with_an_unassigned_child() {
    // A parent explicitly assigned to `bob` claims everything beneath it -
    // an unassigned child is not up for grabs by someone else just because
    // the child itself carries no marker. This guards against the (fixed)
    // regression where a parent's own `@` marker was skipped entirely once
    // it had children, silently making it "eligible" for anyone.
    let input = "\
- [ ] parent task @bob
  - [ ] unassigned subtask
";
    let items = p(input);
    let task = first_task(&items);
    let config = config_with_users(&["alice", "bob"]);
    assert!(!is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("alice".to_string()),
        &config
    ));
    // ...but it's still eligible for the assignee themselves.
    assert!(is_eligible_for(
        NodeRef::Task(task),
        &ResolvedIdentity::Known("bob".to_string()),
        &config
    ));
}

#[test]
fn assignment_on_a_mid_level_task_blocks_eligibility_for_its_whole_branch() {
    // Same cascade rule, deeper: `mid level` claims itself and `deep leaf`
    // for `bob`, so the top-level task has nothing left for `alice`.
    let input = "\
- [ ] some #feature
  - [ ] mid level @bob
    - [ ] deep leaf
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

// ── is_next_task: generalized to all levels, not just literal leaves ──────

#[test]
fn childless_todo_task_is_the_next_task() {
    let input = "\
- [ ] a leaf task
";
    let items = p(input);
    let task = first_task(&items);
    assert!(is_next_task(NodeRef::Task(task), &[], None));
}

#[test]
fn todo_task_with_an_incomplete_child_is_not_itself_the_next_task() {
    // Worked on *through* its child instead - see the actual leaf test below.
    let input = "\
- [ ] parent task
  - [ ] child task
";
    let items = p(input);
    let task = first_task(&items);
    assert!(!is_next_task(NodeRef::Task(task), &[], None));
    assert!(is_next_task(
        NodeRef::Subtask(&task.children[0]),
        &task.children,
        None
    ));
}

#[test]
fn todo_task_whose_children_are_all_done_or_cancelled_is_itself_the_next_task() {
    // Regression test for the reported bug: a task marked todo whose entire
    // subtree has already been resolved (done/cancelled) has nothing left
    // to delegate to, so it becomes the next task in its own right, exactly
    // as if it had no children at all.
    let input = "\
- [ ] parent task
  - [x] done child
  - [-] cancelled child
";
    let items = p(input);
    let task = first_task(&items);
    assert!(is_next_task(NodeRef::Task(task), &[], None));
}

#[test]
fn todo_task_whose_only_incomplete_descendant_is_order_blocked_is_not_the_next_task() {
    // Unlike the all-resolved case above, an order-blocked (but still todo)
    // descendant means there IS remaining work below - just not currently
    // actionable - so the parent must not fall back to being marked itself.
    let input = "\
- [ ] parent task
  - [ ] 1. first step
  - [ ] 2. second step
";
    let items = p(input);
    let task = first_task(&items);
    assert!(!is_next_task(NodeRef::Task(task), &[], None));
}

#[test]
fn deeply_nested_all_resolved_descendants_make_the_mid_level_task_the_next_task() {
    let input = "\
- [ ] grandparent
  - [ ] parent task
    - [x] done child
    - [-] cancelled child
";
    let items = p(input);
    let task = first_task(&items);
    let parent = &task.children[0];
    assert!(is_next_task(NodeRef::Subtask(parent), &task.children, None));
    // The grandparent itself is not next - it still has remaining work
    // below it (the parent task, which is itself now the next task).
    assert!(!is_next_task(NodeRef::Task(task), &[], None));
}

#[test]
fn identity_filtering_still_applies_to_the_all_resolved_fallback() {
    // Same all-resolved-children shape as above, but the parent itself is
    // assigned to bob - alice must not be offered it as her next task.
    let input = "\
- [ ] parent task @bob
  - [x] done child
";
    let items = p(input);
    let task = first_task(&items);
    let config = config_with_users(&["alice", "bob"]);
    let alice = ResolvedIdentity::Known("alice".to_string());
    let bob = ResolvedIdentity::Known("bob".to_string());
    assert!(!is_next_task(
        NodeRef::Task(task),
        &[],
        Some((&alice, &config))
    ));
    assert!(is_next_task(
        NodeRef::Task(task),
        &[],
        Some((&bob, &config))
    ));
}
