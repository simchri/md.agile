use super::*;
use crate::config::{Config, PropertyConfig};
use crate::parser::{FileItem, parse};
use crate::rules::required_title::matches_required_title;
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
                        brief: None,
                        description: None,
                        subtasks: subs.iter().map(|s| s.to_string()).collect(),
                        subtasks_allow_cancel: vec![],
                        neighbortasks: vec![],
                        form: None,
                        branches: Default::default(),
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
                        brief: None,
                        description: None,
                        subtasks: subs.iter().map(|s| s.to_string()).collect(),
                        subtasks_allow_cancel: allow_cancel.to_vec(),
                        neighbortasks: vec![],
                        form: None,
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
fn required_title_accepts_a_resolved_pending_marker() {
    assert!(matches_required_title(
        "check #review:...",
        "check #review:passed"
    ));
}

#[test]
fn required_title_accepts_an_unknown_outcome_for_identity_matching() {
    assert!(matches_required_title(
        "check #review:...",
        "check #review:unknown"
    ));
}

#[test]
fn required_title_accepts_an_unchanged_pending_marker() {
    assert!(matches_required_title(
        "check #review:...",
        "check #review:..."
    ));
}

#[test]
fn required_title_rejects_a_different_explicit_outcome() {
    assert!(!matches_required_title(
        "check #review:passed",
        "check #review:failed"
    ));
}

#[test]
fn required_title_rejects_pending_in_place_of_an_explicit_outcome() {
    assert!(!matches_required_title(
        "check #review:passed",
        "check #review:..."
    ));
}

#[test]
fn required_title_rejects_a_different_property() {
    assert!(!matches_required_title(
        "check #review:...",
        "check #approval:passed"
    ));
}

#[test]
fn required_title_rejects_a_plain_property_in_place_of_pending() {
    assert!(!matches_required_title(
        "check #review:...",
        "check #review"
    ));
}

#[test]
fn required_title_rejects_changed_leading_text() {
    assert!(!matches_required_title(
        "check #review:...",
        "changed #review:passed"
    ));
}

#[test]
fn required_title_rejects_changed_whitespace() {
    assert!(!matches_required_title(
        "check #review:...",
        "check  #review:passed"
    ));
}

#[test]
fn required_title_rejects_changed_order_numbers() {
    assert!(!matches_required_title(
        "1. check @alice #review:...",
        "2. check @alice #review:passed",
    ));
}

#[test]
fn required_title_rejects_changed_assignments() {
    assert!(!matches_required_title(
        "check @alice #review:...",
        "check @bob #review:passed",
    ));
}

#[test]
fn required_title_rejects_changed_marker_order() {
    assert!(!matches_required_title(
        "check #review:... @alice",
        "check @alice #review:passed",
    ));
}

#[test]
fn required_title_does_not_treat_punctuated_pending_text_as_a_placeholder() {
    assert!(!matches_required_title(
        "check #review:...!",
        "check #review:passed!"
    ));
}

#[test]
fn required_title_keeps_code_span_markers_literal() {
    assert!(!matches_required_title(
        "check `#review:...`",
        "check `#review:passed`"
    ));
}

#[test]
fn required_title_keeps_tick_wrapped_markers_literal() {
    assert!(!matches_required_title(
        "check '#review:...'",
        "check '#review:passed'"
    ));
}

#[test]
fn required_title_keeps_escaped_markers_literal() {
    assert!(!matches_required_title(
        r"check \#review:...",
        r"check \#review:passed"
    ));
}

#[test]
fn required_title_rejects_code_span_text_in_place_of_a_marker() {
    assert!(!matches_required_title(
        "check #review:...",
        "check `#review:passed`"
    ));
}

#[test]
fn required_title_rejects_escaped_text_in_place_of_a_marker() {
    assert!(!matches_required_title(
        "check #review:...",
        r"check \#review:passed"
    ));
}

#[test]
fn required_title_accepts_multiple_resolved_markers() {
    assert!(matches_required_title(
        "check #review:... #approval:...",
        "check #review:passed #approval:yes",
    ));
}

#[test]
fn required_title_rejects_a_changed_second_property() {
    assert!(!matches_required_title(
        "check #review:... #approval:...",
        "check #review:passed #other:yes",
    ));
}

#[test]
fn required_title_accepts_repeated_markers_with_distinct_outcomes() {
    assert!(matches_required_title(
        "check #review:... #review:...",
        "check #review:passed #review:failed",
    ));
}

#[test]
fn required_title_accepts_resolved_markers_surrounded_by_unicode_text() {
    assert!(matches_required_title(
        "révision #review:... fin",
        "révision #review:passed fin"
    ));
}

#[test]
fn required_title_rejects_changed_trailing_text() {
    assert!(!matches_required_title(
        "check #review:... done",
        "check #review:passed changed"
    ));
}

#[test]
fn cancellation_of_a_resolved_required_subtask_is_rejected_when_disallowed() {
    let file_content = "\
- [ ] #feature
  - [-] \"check #review:passed\"
";
    let config = config_with_cancellable_subtasks(&[("feature", &["check #review:..."], &[false])]);

    let issues = missing_required_subtasks(&p(file_content), &config);

    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(
        issues[0].code,
        ErrorCode::CancelledRequiredSubtaskNotAllowed
    );
}

#[test]
fn cancellation_of_a_resolved_required_subtask_is_accepted_when_allowed() {
    let file_content = "\
- [ ] #feature
  - [-] \"check #review:passed\"
";
    let config = config_with_cancellable_subtasks(&[("feature", &["check #review:..."], &[true])]);

    let issues = missing_required_subtasks(&p(file_content), &config);

    assert!(issues.is_empty(), "{issues:?}");
}

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
    assert_eq!(issues[0].column, 3); // "developer #review" subtask is indented 2 spaces
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
    assert_eq!(issues[0].column, 3); // "PO review" subtask is indented 2 spaces
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

// ── Branch Properties ──────────────────────────────────────────────────────────

#[test]
fn resolved_branch_adds_its_own_required_subtasks_on_top_of_the_base_property() {
    use crate::config::BranchConfig;
    let mut config = config_with_subtasks(&[("review", &["document review findings"])]);
    config
        .properties
        .get_mut("review")
        .unwrap()
        .branches
        .insert(
            "passed".to_string(),
            BranchConfig {
                subtasks: vec!["notify stakeholders".to_string()],
                ..Default::default()
            },
        );
    let input = "\
- [ ] build something
  - [ ] perform #review:passed
    - [ ] \"document review findings\"
";
    let issues = missing_required_subtasks(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::MissingRequiredSubtasks);
    assert!(
        issues[0].message.contains("notify stakeholders"),
        "{}",
        issues[0].message
    );
}

#[test]
fn pending_branch_property_only_requires_the_base_propertys_own_subtasks() {
    use crate::config::BranchConfig;
    let mut config = config_with_subtasks(&[("review", &["document review findings"])]);
    config
        .properties
        .get_mut("review")
        .unwrap()
        .branches
        .insert(
            "passed".to_string(),
            BranchConfig {
                subtasks: vec!["notify stakeholders".to_string()],
                ..Default::default()
            },
        );
    let input = "\
- [ ] build something
  - [ ] perform #review:...
    - [ ] \"document review findings\"
";
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn undefined_resolved_branch_contributes_no_extra_required_subtasks() {
    // E022 ("Undefined Branch Outcome") handles the invalid branch name
    // itself; this rule simply has nothing extra to require.
    let config = config_with_subtasks(&[("review", &["document review findings"])]);
    let input = "\
- [ ] build something
  - [ ] perform #review:bogus
    - [ ] \"document review findings\"
";
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn resolved_branchs_required_subtask_identical_to_the_base_propertys_own_is_not_duplicated() {
    // The branch and the base property both require "sign off" — a single
    // quoted subtask must satisfy both, not be reported as two instances.
    use crate::config::BranchConfig;
    let mut config = config_with_subtasks(&[("review", &["sign off"])]);
    config
        .properties
        .get_mut("review")
        .unwrap()
        .branches
        .insert(
            "passed".to_string(),
            BranchConfig {
                subtasks: vec!["sign off".to_string()],
                ..Default::default()
            },
        );
    let input = "\
- [ ] build something
  - [ ] perform #review:passed
    - [ ] \"sign off\"
";
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn resolved_branchs_required_subtask_identical_to_a_different_propertys_own_is_not_duplicated() {
    // #feature requires "test", and #review's "passed" branch also requires
    // "test" — a single quoted subtask must satisfy both properties at once.
    use crate::config::BranchConfig;
    let mut config = config_with_subtasks(&[
        ("feature", &["test"]),
        ("review", &["document review findings"]),
    ]);
    config
        .properties
        .get_mut("review")
        .unwrap()
        .branches
        .insert(
            "passed".to_string(),
            BranchConfig {
                subtasks: vec!["test".to_string()],
                ..Default::default()
            },
        );
    let input = "\
- [ ] #feature and #review:passed task
  - [ ] \"document review findings\"
  - [ ] \"test\"
";
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}

#[test]
fn branchs_allow_cancel_merges_with_the_base_propertys_for_an_identical_title() {
    // The base property disallows cancelling "sign off", but the resolved
    // branch's own copy of the same required title allows it — the more
    // permissive setting should win (OR-merge), so cancelling is accepted.
    use crate::config::BranchConfig;
    let config = {
        let mut config = config_with_cancellable_subtasks(&[("review", &["sign off"], &[false])]);
        config
            .properties
            .get_mut("review")
            .unwrap()
            .branches
            .insert(
                "passed".to_string(),
                BranchConfig {
                    subtasks: vec!["sign off".to_string()],
                    subtasks_allow_cancel: vec![true],
                    ..Default::default()
                },
            );
        config
    };
    let input = "\
- [ ] build something
  - [ ] perform #review:passed
    - [-] \"sign off\"
";
    assert!(missing_required_subtasks(&p(input), &config).is_empty());
}
