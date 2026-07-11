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
                        subtasks_allow_cancel: vec![],
                    },
                )
            })
            .collect(),
        ..Config::default()
    }
}

/// Like [`config_with_subtasks`] but each entry also carries a parallel
/// `allow_cancel` array (same length as `subs`).
fn config_with_cancellable_subtasks(entries: &[(&str, &[&str], &[bool])]) -> Config {
    Config {
        properties: entries
            .iter()
            .map(|&(name, subs, allow_cancel)| {
                (
                    name.to_string(),
                    PropertyConfig {
                        name: name.to_string(),
                        subtasks: subs.iter().map(|s| s.to_string()).collect(),
                        subtasks_allow_cancel: allow_cancel.to_vec(),
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

// ── subtasks_allow_cancel ──────────────────────────────────────────────────────

#[test]
fn cancelled_required_subtask_is_satisfied_when_allowed() {
    let input = "\
- [ ] #feature: add basket
  - [-] \"PO review\"
";
    let config = config_with_cancellable_subtasks(&[("feature", &["PO review"], &[true])]);
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn cancelled_required_subtask_flags_e012_when_not_allowed() {
    let input = "\
- [ ] #feature: add basket
  - [-] \"PO review\"
";
    let config = config_with_cancellable_subtasks(&[("feature", &["PO review"], &[false])]);
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].code,
        ErrorCode::CancelledRequiredSubtaskNotAllowed
    );
    assert_eq!(issues[0].location.line, 2);
    assert!(
        issues[0].message.contains("PO review"),
        "{}",
        issues[0].message
    );
}

#[test]
fn cancelled_required_subtask_flags_e012_when_allow_cancel_not_configured() {
    let input = "\
- [ ] #feature: add basket
  - [-] \"PO review\"
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].code,
        ErrorCode::CancelledRequiredSubtaskNotAllowed
    );
}

#[test]
fn mixed_allow_cancel_array_only_flags_disallowed_cancellation() {
    let input = "\
- [ ] #feature: add basket
  - [-] \"PO review\"
  - [-] \"dev implementation\"
";
    let config = config_with_cancellable_subtasks(&[(
        "feature",
        &["PO review", "dev implementation"],
        &[true, false],
    )]);
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].code,
        ErrorCode::CancelledRequiredSubtaskNotAllowed
    );
    assert!(
        issues[0].message.contains("dev implementation"),
        "{}",
        issues[0].message
    );
    assert!(
        !issues[0].message.contains("PO review"),
        "{}",
        issues[0].message
    );
}

#[test]
fn done_required_subtask_is_unaffected_by_allow_cancel_setting() {
    let input = "\
- [x] #feature: add basket
  - [x] \"PO review\"
";
    let config = config_with_cancellable_subtasks(&[("feature", &["PO review"], &[false])]);
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn required_subtask_with_order_prefix_baked_into_config_string_still_matches() {
    // README.vision.md "Ordered Tasks via Properties": a property can declare
    // its required subtasks with an order prefix baked into the literal
    // string, e.g. `subtasks = ["1. dev implementation", "2. dev documentation"]`.
    // The parser now also detects an `Order` from such a subtask (see
    // parser::tests::property_required_subtask_with_order_prefix_is_detected_as_ranked),
    // but this must not break the byte-exact `raw_title` match this rule
    // relies on for E010/E012.
    let input = "\
- [ ] #feature: add basket
  - [ ] \"1. dev implementation\"
  - [ ] \"2. dev documentation\"
";
    let config = config_with_subtasks(&[(
        "feature",
        &["1. dev implementation", "2. dev documentation"],
    )]);
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn missing_required_subtask_with_order_prefix_is_still_reported() {
    let input = "\
- [ ] #feature: add basket
  - [ ] \"1. dev implementation\"
";
    let config = config_with_subtasks(&[(
        "feature",
        &["1. dev implementation", "2. dev documentation"],
    )]);
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::MissingRequiredSubtasks);
    assert!(
        issues[0].message.contains("2. dev documentation"),
        "{}",
        issues[0].message
    );
}
