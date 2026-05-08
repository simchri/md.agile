use super::*;
use crate::config::{Config, PropertyConfig};
use crate::parser::parse;
use std::collections::HashMap;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

fn config_with(names: &[&str]) -> Config {
    Config {
        properties: names
            .iter()
            .map(|&n| {
                (
                    n.to_string(),
                    PropertyConfig {
                        name: n.to_string(),
                    },
                )
            })
            .collect(),
    }
}

#[test]
fn no_issues_when_no_properties_used() {
    let input = "\
- [ ] a plain task with no markers
";
    let issues = undefined_property(&p(input), &Config::default());
    assert!(issues.is_empty());
}

#[test]
fn no_issues_when_properties_quoted() {
    let input = "\
- [ ] a plain task with no actual markers, but a quoted '#marker'
";
    let issues = undefined_property(&p(input), &Config::default());
    assert!(issues.is_empty());
}

#[test]
fn no_issues_when_property_is_defined() {
    let input = "\
- [ ] a task with #feature marker
";
    let issues = undefined_property(&p(input), &config_with(&["feature"]));
    assert!(issues.is_empty());
}

#[test]
fn flags_undefined_property_even_with_no_config_file() {
    let input = "\
- [ ] a task with #feature marker
";
    let issues = undefined_property(&p(input), &Config::default());
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UndefinedProperty);
    assert!(
        issues[0].message.contains("feature"),
        "{}",
        issues[0].message
    );
}

#[test]
fn flags_undefined_property_on_task() {
    let input = "\
- [ ] a task with #feature marker
";
    let issues = undefined_property(
        &p(input),
        &Config {
            properties: HashMap::new(),
        },
    );
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UndefinedProperty);
    assert!(
        issues[0].message.contains("feature"),
        "{}",
        issues[0].message
    );
}

#[test]
fn flags_undefined_property_on_subtask() {
    let input = "\
- [ ] a task
  - [ ] a subtask with #bug marker
";
    let issues = undefined_property(&p(input), &Config::default());
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 2);
}

#[test]
fn special_markers_are_not_flagged() {
    let input = "\
- [ ] a task
  - [ ] #OPT optional subtask
";
    let issues = undefined_property(&p(input), &Config::default());
    assert!(issues.is_empty(), "special markers must not be flagged");
}

#[test]
fn multiple_undefined_properties_each_produce_an_issue() {
    let input = "\
- [ ] #feature and #bug task
";
    let issues = undefined_property(&p(input), &Config::default());
    assert_eq!(issues.len(), 2);
}

#[test]
fn mix_of_defined_and_undefined_only_flags_undefined() {
    let input = "\
- [ ] #feature and #bug task
";
    let issues = undefined_property(&p(input), &config_with(&["feature"]));
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("bug"), "{}", issues[0].message);
}

#[test]
fn branch_form_property_is_also_checked() {
    let input = "\
- [ ] perform #review...
";
    let issues = undefined_property(&p(input), &Config::default());
    assert_eq!(issues.len(), 1);
    assert!(
        issues[0].message.contains("review"),
        "{}",
        issues[0].message
    );
}
