use super::*;
use crate::config::{BranchConfig, Config, PropertyConfig};
use crate::parser::parse;
use std::collections::HashMap;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

fn config_with_branches(property: &str, branches: &[&str]) -> Config {
    let mut props = HashMap::new();
    props.insert(
        property.to_string(),
        PropertyConfig {
            name: property.to_string(),
            brief: None,
            description: None,
            subtasks: vec![],
            subtasks_allow_cancel: vec![],
            neighbortasks: vec![],
            form: None,
            branches: branches
                .iter()
                .map(|&b| (b.to_string(), BranchConfig::default()))
                .collect(),
        },
    );
    Config {
        properties: props,
        ..Config::default()
    }
}

// ── E021 "Unresolved Branch Property" ──────────────────────────────────────────

#[test]
fn no_issue_when_pending_property_is_not_done() {
    let input = "\
- [ ] perform #review:...
";
    assert!(unresolved_branch_property(&p(input)).is_empty());
}

#[test]
fn no_issue_when_resolved_property_is_done() {
    let input = "\
- [x] perform #review:passed
";
    assert!(unresolved_branch_property(&p(input)).is_empty());
}

#[test]
fn no_issue_when_done_task_has_no_branch_property() {
    let input = "\
- [x] perform #review
";
    assert!(unresolved_branch_property(&p(input)).is_empty());
}

#[test]
fn flags_done_task_with_pending_branch_property() {
    let input = "\
- [x] perform #review:...
";
    let issues = unresolved_branch_property(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UnresolvedBranchProperty);
    assert!(issues[0].message.contains("#review:..."));
    assert_eq!(
        issues[0].data,
        Some(crate::rules::IssueData::UnresolvedBranchProperty {
            properties: vec!["review".to_string()],
        })
    );
}

#[test]
fn flags_done_subtask_with_pending_branch_property() {
    let input = "\
- [ ] build something
  - [x] perform #review:...
";
    let issues = unresolved_branch_property(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 2);
}

#[test]
fn does_not_flag_a_cancelled_task_with_an_unresolved_branch_property() {
    // Cancelling a task is a deliberate way to abandon it without resolving
    // its outcome — only completing it (Done) requires resolution.
    let input = "\
- [-] perform #review:...
";
    assert!(unresolved_branch_property(&p(input)).is_empty());
}

#[test]
fn flags_done_task_with_multiple_unresolved_branch_properties() {
    let input = "\
- [x] perform #review:... and #approval:...
";
    let issues = unresolved_branch_property(&p(input));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UnresolvedBranchProperty);
    assert!(issues[0].message.contains("#review:..."));
    assert!(issues[0].message.contains("#approval:..."));
    assert!(
        issues[0].message.contains("properties"),
        "message should pluralize when more than one branch property is unresolved: {}",
        issues[0].message
    );
    assert_eq!(
        issues[0].data,
        Some(crate::rules::IssueData::UnresolvedBranchProperty {
            properties: vec!["review".to_string(), "approval".to_string()],
        })
    );
}

#[test]
fn check_unresolved_flags_pending_property_regardless_of_status() {
    // Used by check_completable, which validates a node that's still `Todo`
    // and about to *become* done — see the function's own doc comment.
    let input = "\
- [ ] perform #review:...
";
    let task = match &p(input)[0] {
        FileItem::Task(t) => t.clone(),
        _ => panic!("expected a task"),
    };
    let issues = check_unresolved(&task.markers, &task.location, task.indent);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UnresolvedBranchProperty);
}

// ── E022 "Undefined Branch Outcome" ────────────────────────────────────────────

#[test]
fn no_issue_when_resolved_branch_is_defined() {
    let input = "\
- [ ] perform #review:passed
";
    let config = config_with_branches("review", &["passed", "failed"]);
    assert!(undefined_branch_outcome(&p(input), &config).is_empty());
}

#[test]
fn no_issue_for_pending_property() {
    let input = "\
- [ ] perform #review:...
";
    let config = config_with_branches("review", &["passed", "failed"]);
    assert!(undefined_branch_outcome(&p(input), &config).is_empty());
}

#[test]
fn no_issue_when_property_is_entirely_undefined() {
    // Left to E008 "Undefined Property" instead.
    let input = "\
- [ ] perform #review:passed
";
    assert!(undefined_branch_outcome(&p(input), &Config::default()).is_empty());
}

#[test]
fn flags_resolved_branch_not_declared_in_config() {
    let input = "\
- [ ] perform #review:bogus
";
    let config = config_with_branches("review", &["passed", "failed"]);
    let issues = undefined_branch_outcome(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UndefinedBranchOutcome);
    assert!(issues[0].message.contains("bogus"));
    assert!(issues[0].message.contains("#review"));
}

#[test]
fn flags_resolved_branch_when_property_declares_no_branches_at_all() {
    let input = "\
- [ ] perform #review:passed
";
    let config = config_with_branches("review", &[]);
    let issues = undefined_branch_outcome(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UndefinedBranchOutcome);
}

// ── E023 "Plain Branching Property" ────────────────────────────────────────────

#[test]
fn no_issue_for_pending_form_on_branching_property() {
    let input = "\
- [ ] perform #review:...
";
    let config = config_with_branches("review", &["passed", "failed"]);
    assert!(plain_branching_property(&p(input), &config).is_empty());
}

#[test]
fn no_issue_for_resolved_form_on_branching_property() {
    let input = "\
- [ ] perform #review:passed
";
    let config = config_with_branches("review", &["passed", "failed"]);
    assert!(plain_branching_property(&p(input), &config).is_empty());
}

#[test]
fn no_issue_for_plain_form_when_property_declares_no_branches() {
    let input = "\
- [ ] perform #review
";
    let config = config_with_branches("review", &[]);
    assert!(plain_branching_property(&p(input), &config).is_empty());
}

#[test]
fn no_issue_for_plain_form_when_property_is_entirely_undefined() {
    // Left to E008 "Undefined Property" instead.
    let input = "\
- [ ] perform #review
";
    assert!(plain_branching_property(&p(input), &Config::default()).is_empty());
}

#[test]
fn flags_plain_form_on_property_that_declares_branches() {
    let input = "\
- [ ] perform #review
";
    let config = config_with_branches("review", &["passed", "failed"]);
    let issues = plain_branching_property(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::PlainBranchingProperty);
    assert!(issues[0].message.contains("#review"));
    assert!(issues[0].message.contains("declares branches"));
}

#[test]
fn flags_plain_form_regardless_of_status() {
    let input = "\
- [x] perform #review
";
    let config = config_with_branches("review", &["passed", "failed"]);
    let issues = plain_branching_property(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::PlainBranchingProperty);
}

#[test]
fn plain_form_issue_points_at_the_marker_column() {
    let input = "\
- [ ] perform #review
";
    let config = config_with_branches("review", &["passed", "failed"]);
    let issues = plain_branching_property(&p(input), &config);
    assert_eq!(issues.len(), 1);
    // `- [ ] perform #review` — the `#` sits at column 15 (1-based).
    assert_eq!(issues[0].column, 15);
}

#[test]
fn check_plain_form_flags_plain_property_directly() {
    let input = "\
- [ ] perform #review
";
    let task = match &p(input)[0] {
        FileItem::Task(t) => t.clone(),
        _ => panic!("expected a task"),
    };
    let config = config_with_branches("review", &["passed", "failed"]);
    let issues = check_plain_form(&task.markers, &task.location, task.indent, &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::PlainBranchingProperty);
}
