use super::*;
use crate::config::{Config, GroupConfig, UserConfig};
use crate::parser::{FileItem, parse};
use std::collections::HashMap;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

fn config_with_users(names: &[&str]) -> Config {
    Config {
        users: names
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

fn config_with_groups(names: &[&str]) -> Config {
    Config {
        groups: names
            .iter()
            .map(|&n| {
                (
                    n.to_string(),
                    GroupConfig {
                        name: n.to_string(),
                        members: vec![],
                    },
                )
            })
            .collect(),
        ..Config::default()
    }
}

#[test]
fn no_issues_when_no_assignment_markers_used() {
    let input = "\
- [ ] a plain task with no markers
";
    let issues = undefined_assignment(&p(input), &Config::default());
    assert!(issues.is_empty());
}

#[test]
fn no_issues_when_user_is_declared() {
    let input = "\
- [ ] implement @alice
";
    let issues = undefined_assignment(&p(input), &config_with_users(&["alice"]));
    assert!(issues.is_empty());
}

#[test]
fn no_issues_when_group_is_declared() {
    let input = "\
- [ ] implement @devs
";
    let issues = undefined_assignment(&p(input), &config_with_groups(&["devs"]));
    assert!(issues.is_empty());
}

#[test]
fn flags_undefined_assignment_with_empty_config() {
    let input = "\
- [ ] implement @alice
";
    let issues = undefined_assignment(&p(input), &Config::default());
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::UndefinedAssignment);
    assert!(issues[0].message.contains("alice"), "{}", issues[0].message);
}

#[test]
fn flags_undefined_assignment_when_only_unrelated_users_declared() {
    let input = "\
- [ ] implement @alice
";
    let issues = undefined_assignment(&p(input), &config_with_users(&["bob"]));
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("alice"), "{}", issues[0].message);
}

#[test]
fn flags_undefined_assignment_on_subtask() {
    let input = "\
- [ ] a task
  - [ ] a subtask @alice
";
    let issues = undefined_assignment(&p(input), &Config::default());
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 2);
}

#[test]
fn multiple_undefined_assignments_each_produce_an_issue() {
    let input = "\
- [ ] implement @alice @bob
";
    let issues = undefined_assignment(&p(input), &Config::default());
    assert_eq!(issues.len(), 2);
}

#[test]
fn mix_of_defined_and_undefined_only_flags_undefined() {
    let input = "\
- [ ] implement @alice @bob
";
    let issues = undefined_assignment(&p(input), &config_with_users(&["alice"]));
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("bob"), "{}", issues[0].message);
}

#[test]
fn user_and_group_with_same_name_does_not_duplicate_issue() {
    let input = "\
- [ ] implement @devs
";
    // Declared as both user and group — no issue.
    let config = Config {
        users: HashMap::from([(
            "devs".to_string(),
            UserConfig {
                name: "devs".to_string(),
                git_emails: vec![],
                git_names: vec![],
            },
        )]),
        groups: HashMap::from([(
            "devs".to_string(),
            GroupConfig {
                name: "devs".to_string(),
                members: vec![],
            },
        )]),
        ..Config::default()
    };
    let issues = undefined_assignment(&p(input), &config);
    assert!(issues.is_empty());
}

#[test]
fn diagnostic_column_points_to_at_marker() {
    let input = "\
- [ ] implement @alice
";
    // "- [ ] implement @alice"
    //  0         1
    //  0123456789012345678901
    // '@alice' starts at column 17 (1-based)
    // indent=0, prefix "- [ ] " = 6, '@alice' at title pos 10 → col=11
    // full_column = 0 + 6 + 11 = 17
    let issues = undefined_assignment(&p(input), &Config::default());
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].column, 17,
        "Column should point to the '@' of @alice"
    );
}
