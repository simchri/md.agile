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

// ── no-issue cases ────────────────────────────────────────────────────────────

#[test]
fn no_issues_when_quoted_subtask_matches_required_by_property() {
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    assert!(unrequired_quoted_subtask(&p(file_content), &config).is_empty());
}

#[test]
fn no_issues_when_all_quoted_subtasks_are_required() {
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
  - [ ] \"dev implementation\"
";
    let config = config_with_subtasks(&[("feature", &["PO review", "dev implementation"])]);
    assert!(unrequired_quoted_subtask(&p(file_content), &config).is_empty());
}

#[test]
fn no_issues_when_task_has_no_quoted_subtasks() {
    let file_content = "\
- [ ] plain task
  - [ ] some custom subtask
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    assert!(unrequired_quoted_subtask(&p(file_content), &config).is_empty());
}

#[test]
fn no_issues_when_custom_subtask_happens_to_use_text_from_property() {
    // An unquoted subtask is never PropertyRequired, so it cannot be E011.
    let file_content = "\
- [ ] plain task
  - [ ] PO review
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    assert!(unrequired_quoted_subtask(&p(file_content), &config).is_empty());
}

#[test]
fn no_issues_for_required_subtask_from_one_of_multiple_properties() {
    let file_content = "\
- [ ] #feature and #UI task
  - [ ] \"PO review\"
  - [ ] \"UI concept\"
";
    let config = config_with_subtasks(&[("feature", &["PO review"]), ("UI", &["UI concept"])]);
    assert!(unrequired_quoted_subtask(&p(file_content), &config).is_empty());
}

// ── error cases ───────────────────────────────────────────────────────────────

#[test]
fn flags_quoted_subtask_not_declared_by_property() {
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"not declared\"
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    let issues = unrequired_quoted_subtask(&p(file_content), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UnrequiredQuotedSubtask);
    assert!(
        issues[0].message.contains("not declared"),
        "{}",
        issues[0].message
    );
}

#[test]
fn flags_quoted_subtask_on_task_with_no_properties() {
    let file_content = "\
- [ ] plain task
  - [ ] \"quoted without property\"
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    let issues = unrequired_quoted_subtask(&p(file_content), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UnrequiredQuotedSubtask);
}

#[test]
fn flags_quoted_subtask_when_property_has_no_subtasks_configured() {
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
";
    // #feature exists but declares no required subtasks.
    let config = config_with_subtasks(&[("feature", &[])]);
    let issues = unrequired_quoted_subtask(&p(file_content), &config);
    assert_eq!(issues.len(), 1);
}

#[test]
fn flags_only_unrequired_among_multiple_quoted_subtasks() {
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
  - [ ] \"sneaky extra\"
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    let issues = unrequired_quoted_subtask(&p(file_content), &config);
    assert_eq!(issues.len(), 1);
    assert!(
        issues[0].message.contains("sneaky extra"),
        "{}",
        issues[0].message
    );
}

#[test]
fn issue_is_reported_at_the_subtask_line() {
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"not declared\"
";
    let config = config_with_subtasks(&[("feature", &["PO review"])]);
    let issues = unrequired_quoted_subtask(&p(file_content), &config);
    // Parser uses 1-based lines; subtask is on source line 2.
    assert_eq!(issues[0].location.line, 2);
}

// ── top-level task carrying the PropertyRequired kind ─────────────────────────

#[test]
fn top_level_quoted_task_is_not_flagged_by_this_rule() {
    // E011 covers *subtasks* (children of a parent node). A top-level
    // `- [ ] "quoted"` cannot be a required subtask in any meaningful sense
    // and is outside the scope of E011 (the parser drops the SubtaskKind when
    // converting a stack frame to a top-level Task).
    let file_content = "\
- [ ] \"orphan quoted task\"
";
    let config = config_with_subtasks(&[("feature", &["orphan quoted task"])]);
    let issues = unrequired_quoted_subtask(&p(file_content), &config);
    assert_eq!(issues.len(), 0);
}

// ── nested subtasks ───────────────────────────────────────────────────────────

#[test]
fn flags_unrequired_quoted_grandchild() {
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
  - [ ] \"developer #review\"
    - [ ] \"spurious grandchild\"
";
    let config = config_with_subtasks(&[
        ("feature", &["PO review", "developer #review"]),
        ("review", &["independent review"]),
    ]);
    // "developer #review" has a #review property requiring "independent review",
    // but "spurious grandchild" is not that → E011 on line 4.
    let issues = unrequired_quoted_subtask(&p(file_content), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 4);
    assert!(
        issues[0].message.contains("spurious grandchild"),
        "{}",
        issues[0].message
    );
}

#[test]
fn no_issues_for_correct_nested_required_subtask() {
    let file_content = "\
- [ ] #feature: add basket
  - [ ] \"PO review\"
  - [ ] \"developer #review\"
    - [ ] \"independent review\"
";
    let config = config_with_subtasks(&[
        ("feature", &["PO review", "developer #review"]),
        ("review", &["independent review"]),
    ]);
    assert!(unrequired_quoted_subtask(&p(file_content), &config).is_empty());
}

// ── issue data ────────────────────────────────────────────────────────────────

#[test]
fn issue_data_carries_raw_title() {
    let file_content = "\
- [ ] plain task
  - [ ] \"my extra\"
";
    let config = config_with_subtasks(&[]);
    let issues = unrequired_quoted_subtask(&p(file_content), &config);
    assert_eq!(
        issues[0].data,
        Some(IssueData::UnrequiredQuotedSubtask {
            title: "my extra".to_string()
        })
    );
}
