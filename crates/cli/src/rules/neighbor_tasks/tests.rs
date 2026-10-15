use super::*;
use crate::config::{Config, PropertyConfig};
use crate::parser::{FileItem, parse};
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

fn config_with_neighbors(entries: &[(&str, &[&str])]) -> Config {
    Config {
        properties: entries
            .iter()
            .map(|&(name, neighbors)| {
                (
                    name.to_string(),
                    PropertyConfig {
                        name: name.to_string(),
                        brief: None,
                        description: None,
                        subtasks: vec![],
                        subtasks_allow_cancel: vec![],
                        neighbortasks: neighbors.iter().map(|s| s.to_string()).collect(),
                        branches: Default::default(),
                    },
                )
            })
            .collect(),
        ..Config::default()
    }
}

// ── no-issue cases ────────────────────────────────────────────────────────────

#[test]
fn no_issues_when_property_has_no_neighbortasks() {
    let input = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    let config = config_with_neighbors(&[("frontend-implementation", &[])]);
    assert!(neighbor_tasks(&p(input), &config).is_empty());
}

#[test]
fn no_issues_when_node_has_no_properties() {
    let input = "\
- [ ] let's build a feature and
  - [ ] plain subtask
";
    let config = config_with_neighbors(&[("frontend-implementation", &["back end"])]);
    assert!(neighbor_tasks(&p(input), &config).is_empty());
}

#[test]
fn no_issues_when_required_neighbor_is_present() {
    let input = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
  - [ ] \"do the back end implementation also!\"
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    assert!(neighbor_tasks(&p(input), &config).is_empty());
}

// ── missing-neighbor cases ─────────────────────────────────────────────────────

#[test]
fn flags_missing_neighbor_task() {
    let input = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    let issues = neighbor_tasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::MissingNeighborTask);
    assert!(
        issues[0]
            .message
            .contains("do the back end implementation also!")
    );
}

#[test]
fn flags_multiple_missing_neighbor_tasks_in_one_issue() {
    let input = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    let config = config_with_neighbors(&[("frontend-implementation", &["backend", "QA sign-off"])]);
    let issues = neighbor_tasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("backend"));
    assert!(issues[0].message.contains("QA sign-off"));
}

#[test]
fn only_missing_neighbor_is_flagged_when_some_present() {
    let input = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
  - [ ] \"backend\"
";
    let config = config_with_neighbors(&[("frontend-implementation", &["backend", "QA sign-off"])]);
    let issues = neighbor_tasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert!(!issues[0].message.contains("\"backend\""));
    assert!(issues[0].message.contains("QA sign-off"));
}

#[test]
fn unquoted_sibling_with_matching_text_does_not_satisfy_requirement() {
    // The neighbor task must use the quoted PropertyRequired syntax, same as
    // regular required subtasks; a plain unquoted sibling doesn't count.
    let input = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
  - [ ] do the back end implementation also!
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    let issues = neighbor_tasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
}

#[test]
fn issue_is_reported_at_the_property_carrying_node_location() {
    let input = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    let issues = neighbor_tasks(&p(input), &config);
    assert_eq!(issues[0].location.line, 2);
}

#[test]
fn issue_data_lists_missing_neighbor_tasks() {
    let input = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    let issues = neighbor_tasks(&p(input), &config);
    assert_eq!(
        issues[0].data,
        Some(IssueData::MissingNeighborTask {
            missing: vec!["do the back end implementation also!".to_string()]
        })
    );
}

// ── top-level restriction ──────────────────────────────────────────────────────

#[test]
fn property_with_neighbortasks_at_top_level_is_not_flagged_by_this_rule() {
    // A top-level Task has no quotable siblings at all, so a neighbortasks
    // property set at the top level can never be satisfied (see
    // README.vision.md: "Properties with Neighbor Tasks can not be set at
    // the top level") -- but that's reported by the more specific E020
    // "Neighbor Task On Top Level" rule instead; this rule skips top-level
    // tasks entirely so the two don't both fire.
    let input = "\
- [ ] #frontend-implementation: top level feature
- [ ] \"do the back end implementation also!\"
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    assert!(neighbor_tasks(&p(input), &config).is_empty());
}

// ── nesting ────────────────────────────────────────────────────────────────────

#[test]
fn nested_property_on_subtask_is_checked_against_its_own_siblings() {
    let input = "\
- [ ] outer task
  - [ ] \"PO review\"
  - [ ] ..first do the #frontend-implementation
  - [ ] \"nested sibling\"
";
    let config = config_with_neighbors(&[("frontend-implementation", &["nested sibling"])]);
    assert!(neighbor_tasks(&p(input), &config).is_empty());
}

#[test]
fn nested_property_missing_its_own_sibling_is_flagged_even_though_an_outer_sibling_matches() {
    // The required title only exists as a child of the property-carrying
    // subtask, not among its own direct siblings -- doesn't count.
    let input = "\
- [ ] outer task
  - [ ] ..first do the #frontend-implementation
    - [ ] \"do the back end implementation also!\"
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    let issues = neighbor_tasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
}

#[test]
fn multiple_properties_merge_required_neighbors_without_duplicates() {
    let input = "\
- [ ] outer task
  - [ ] #frontend-implementation and #backend-implementation task
  - [ ] \"shared neighbor\"
";
    let config = config_with_neighbors(&[
        ("frontend-implementation", &["shared neighbor"]),
        ("backend-implementation", &["shared neighbor"]),
    ]);
    assert!(neighbor_tasks(&p(input), &config).is_empty());
}

// ── Branch Properties ──────────────────────────────────────────────────────────

#[test]
fn resolved_branch_adds_its_own_required_neighbor_on_top_of_the_base_property() {
    use crate::config::BranchConfig;
    let mut config = Config::default();
    config.properties.insert(
        "review".to_string(),
        PropertyConfig {
            name: "review".to_string(),
            brief: None,
            description: None,
            subtasks: vec![],
            subtasks_allow_cancel: vec![],
            neighbortasks: vec![],
            branches: [(
                "passed".to_string(),
                BranchConfig {
                    neighbortasks: vec!["publish feature".to_string()],
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        },
    );
    let input = "\
- [ ] build something
  - [ ] perform #review:passed
  - [ ] \"publish feature\"
";
    assert!(neighbor_tasks(&p(input), &config).is_empty());

    let input_missing = "\
- [ ] build something
  - [ ] perform #review:passed
";
    let issues = neighbor_tasks(&p(input_missing), &config);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("publish feature"));
}

#[test]
fn pending_branch_property_does_not_yet_require_an_unchosen_branchs_neighbor() {
    use crate::config::BranchConfig;
    let mut config = Config::default();
    config.properties.insert(
        "review".to_string(),
        PropertyConfig {
            name: "review".to_string(),
            brief: None,
            description: None,
            subtasks: vec![],
            subtasks_allow_cancel: vec![],
            neighbortasks: vec![],
            branches: [(
                "passed".to_string(),
                BranchConfig {
                    neighbortasks: vec!["publish feature".to_string()],
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        },
    );
    let input = "\
- [ ] build something
  - [ ] perform #review...
";
    assert!(neighbor_tasks(&p(input), &config).is_empty());
}
