use super::*;
use crate::config::{Config, PropertyConfig};
use crate::parser::parse;
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
- [ ] #frontend-implementation: top level feature
";
    let config = config_with_neighbors(&[("frontend-implementation", &[])]);
    assert!(neighbor_task_on_top_level(&p(input), &config).is_empty());
}

#[test]
fn no_issues_when_top_level_task_has_no_properties() {
    let input = "\
- [ ] plain top level task
";
    let config = config_with_neighbors(&[("frontend-implementation", &["back end"])]);
    assert!(neighbor_task_on_top_level(&p(input), &config).is_empty());
}

#[test]
fn no_issues_when_property_with_neighbortasks_is_on_a_subtask() {
    let input = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
  - [ ] \"do the back end implementation also!\"
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    assert!(neighbor_task_on_top_level(&p(input), &config).is_empty());
}

// ── flagged cases ─────────────────────────────────────────────────────────────

#[test]
fn flags_top_level_task_carrying_a_neighbortasks_property() {
    let input = "\
- [ ] #frontend-implementation: top level feature
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    let issues = neighbor_task_on_top_level(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::NeighborTaskOnTopLevel);
    assert!(issues[0].message.contains("#frontend-implementation"));
}

#[test]
fn flags_even_when_a_quoted_sibling_happens_to_exist() {
    // Irrelevant whether a quoted "neighbor" sibling is present: top-level
    // tasks can't carry this kind of property at all, regardless.
    let input = "\
- [ ] #frontend-implementation: top level feature
- [ ] \"do the back end implementation also!\"
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    let issues = neighbor_task_on_top_level(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::NeighborTaskOnTopLevel);
}

#[test]
fn flags_each_offending_top_level_task_independently() {
    let input = "\
- [ ] #frontend-implementation: first top level feature
- [ ] #frontend-implementation: second top level feature
";
    let config = config_with_neighbors(&[(
        "frontend-implementation",
        &["do the back end implementation also!"],
    )]);
    let issues = neighbor_task_on_top_level(&p(input), &config);
    assert_eq!(issues.len(), 2);
    assert!(
        issues
            .iter()
            .all(|i| i.code == ErrorCode::NeighborTaskOnTopLevel)
    );
}

#[test]
fn names_multiple_offending_properties_in_the_message() {
    let input = "\
- [ ] #frontend-implementation #design-review: top level feature
";
    let config = config_with_neighbors(&[
        ("frontend-implementation", &["back end"]),
        ("design-review", &["QA sign-off"]),
    ]);
    let issues = neighbor_task_on_top_level(&p(input), &config);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("#frontend-implementation"));
    assert!(issues[0].message.contains("#design-review"));
}
