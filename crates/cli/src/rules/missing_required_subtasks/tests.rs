use super::*;
use crate::config::{Config, PropertyConfig};
use crate::parser::{FileItem, parse};
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

fn config_with_subtasks(entries: &[(&str, &[&str])]) -> Config {
    Config {
        properties: entries
            .iter()
            .map(|&(name, subs)| {
                (
                    name.to_string(),
                    PropertyConfig {
                        name: name.to_string(),
                        subtasks: subs.iter().map(|s| s.to_string()).collect(),
                    },
                )
            })
            .collect(),
        ..Config::default()
    }
}

// ── no-issue cases ────────────────────────────────────────────────────────────

#[test]
fn no_issues_when_property_has_no_subtasks() {
    let input = "\
- [ ] #feature: add basket
";
    let config = config_with_subtasks(&[("feature", &[])]);
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn no_issues_when_task_has_no_properties() {
    let input = "\
- [ ] plain task
  - [ ] \"PO review\"
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn no_issues_when_all_required_subtasks_present() {
    let input = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
  - [ ] \"dev implementation\"
";
    let config = config_with_subtasks(&[("feature", &["PO review", "dev implementation"])]);
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn no_issues_when_extra_custom_subtasks_are_present() {
    let input = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
  - [ ] some unquoted custom task
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

// ── error cases ───────────────────────────────────────────────────────────────

#[test]
fn flags_single_missing_subtask() {
    let input = "\
- [ ] #feature: add basket
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::MissingRequiredSubtasks);
    assert!(
        issues[0].message.contains("PO review"),
        "{}",
        issues[0].message
    );
}

#[test]
fn flags_multiple_missing_subtasks_in_one_issue() {
    let input = "\
- [ ] #feature: add basket
";
    let config = config_with_subtasks(&[("feature", &["PO review", "dev implementation", "test"])]);
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert!(
        issues[0].message.contains("PO review"),
        "{}",
        issues[0].message
    );
    assert!(
        issues[0].message.contains("dev implementation"),
        "{}",
        issues[0].message
    );
    assert!(issues[0].message.contains("test"), "{}", issues[0].message);
}

#[test]
fn only_missing_subtasks_are_flagged_not_present_ones() {
    let input = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
";
    let config = config_with_subtasks(&[("feature", &["PO review", "test"])]);
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("test"), "{}", issues[0].message);
    assert!(
        !issues[0].message.contains("PO review"),
        "{}",
        issues[0].message
    );
}

#[test]
fn issue_is_reported_at_task_location() {
    let input = "\
- [ ] #feature: add basket
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(issues[0].location.line, 1);
}

#[test]
fn issue_data_lists_missing_subtasks() {
    let input = "\
- [ ] #feature: add basket
";
    let config = config_with_subtasks(&[("feature", &["PO review", "test"])]);
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(
        issues[0].data,
        Some(IssueData::MissingRequiredSubtasks {
            missing: vec!["PO review".to_string(), "test".to_string()]
        })
    );
}

// ── multiple properties ───────────────────────────────────────────────────────

#[test]
fn multiple_properties_all_present_no_issue() {
    let input = "\
- [ ] #feature and #UI task
  - [ ] \"PO review\"
  - [ ] \"dev implementation\"
  - [ ] \"UI concept\"
";
    let config = config_with_subtasks(&[
        ("feature", &["PO review", "dev implementation"]),
        ("UI", &["UI concept"]),
    ]);
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn multiple_properties_missing_from_second_property() {
    let input = "\
- [ ] #feature and #UI task
  - [ ] \"PO review\"
  - [ ] \"dev implementation\"
";
    let config = config_with_subtasks(&[
        ("feature", &["PO review", "dev implementation"]),
        ("UI", &["UI concept"]),
    ]);
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert!(
        issues[0].message.contains("UI concept"),
        "{}",
        issues[0].message
    );
}

#[test]
fn shared_required_subtask_across_two_properties_not_duplicated() {
    // Both #feature and #UI require "test" — it should only appear once in the required list.
    let input = "\
- [ ] #feature and #UI task
  - [ ] \"test\"
";
    let config = config_with_subtasks(&[("feature", &["test"]), ("UI", &["test"])]);
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

// ── nested properties ─────────────────────────────────────────────────────────

#[test]
fn nested_property_on_subtask_is_checked() {
    // The #review property requires "independent review".
    // A PropertyRequired subtask "developer #review" carries the #review property
    // and must itself have the required child.
    let input = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
  - [ ] \"developer #review\"
";
    let config = config_with_subtasks(&[
        ("feature", &["PO review", "developer #review"]),
        ("review", &["independent review"]),
    ]);
    let issues = missing_required_subtasks(&p(input), &config);
    // The top-level task is satisfied; the "developer #review" subtask is missing its child.
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 3);
    assert!(
        issues[0].message.contains("independent review"),
        "{}",
        issues[0].message
    );
}

#[test]
fn nested_property_fully_satisfied_no_issue() {
    let input = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
  - [ ] \"developer #review\"
    - [ ] \"independent review\"
";
    let config = config_with_subtasks(&[
        ("feature", &["PO review", "developer #review"]),
        ("review", &["independent review"]),
    ]);
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}
