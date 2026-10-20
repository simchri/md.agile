use super::*;
use crate::parser::parse;
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn branch_forms_activate_additively_and_only_for_the_selected_outcome() {
    let config = crate::config::Config::from_str(
        "\
[Field.base]
label = \"Base\"
[Field.approval]
label = \"Approval\"
[Field.rejection]
label = \"Rejection\"
[Form.base]
fields = [\"base\"]
[Form.approval]
fields = [\"approval\"]
[Form.rejection]
fields = [\"rejection\"]
[Properties.review]
form = \"base\"
[Properties.review.passed]
form = \"approval\"
[Properties.review.failed]
form = \"rejection\"
",
    )
    .unwrap();
    for (outcome, expected) in [
        ("...", vec!["Base"]),
        ("unknown", vec!["Base"]),
        ("passed", vec!["Base", "Approval"]),
        ("failed", vec!["Base", "Rejection"]),
    ] {
        let file_content = format!(
            "\
- [ ] #review:{outcome}
"
        );

        let issues = missing_required_field(&p(&file_content), &config);
        let missing: Vec<&str> = issues
            .iter()
            .flat_map(|issue| match &issue.data {
                Some(IssueData::MissingRequiredField { missing }) => {
                    missing.iter().map(String::as_str).collect()
                }
                _ => vec![],
            })
            .collect();

        assert_eq!(missing, expected, "{outcome}");
    }
}

#[test]
fn branch_form_fields_use_all_field_validation_rules() {
    let config = crate::config::Config::from_str(
        "\
[Field.answer]
label = \"Answer\"
values = [\"yes\"]
[Field.bio]
label = \"Bio\"
multiline = true
[Form.details]
fields = [\"answer\", \"bio\"]
[Properties.review]
[Properties.review.passed]
form = \"details\"
",
    )
    .unwrap();
    let file_content = "\
- [x] #review:passed
  __Answer:__ no
  __Bio:__ ___
";

    let issues = check_all(&p(file_content), &config);

    let codes: Vec<_> = issues.iter().map(|issue| issue.code).collect();
    assert!(
        codes.contains(&ErrorCode::FieldValueNotAllowed),
        "{issues:?}"
    );
    assert!(
        codes.contains(&ErrorCode::MultilineFieldFilledInline),
        "{issues:?}"
    );
    assert!(codes.contains(&ErrorCode::FieldLeftUnfilled), "{issues:?}");
}

#[test]
fn branch_forms_deduplicate_identical_forms_and_detect_base_branch_collisions() {
    let config = crate::config::Config::from_str(
        "\
[Field.name]
label = \"Name\"
[Field.other_name]
label = \"Name\"
[Form.base]
fields = [\"name\"]
[Form.other]
fields = [\"other_name\"]
[Properties.review]
form = \"base\"
[Properties.review.same]
form = \"base\"
[Properties.review.different]
form = \"other\"
[Properties.shared]
form = \"base\"
",
    )
    .unwrap();
    let file_content = "\
- [ ] #review:same #shared #review:same
";

    let issues = missing_required_field(&p(file_content), &config);

    assert_eq!(issues.len(), 1, "{issues:?}");
    assert!(colliding_field_labels(&p(file_content), &config).is_empty());

    let file_content = "\
- [ ] #review:different
";
    let issues = colliding_field_labels(&p(file_content), &config);
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].code, ErrorCode::CollidingFieldLabels);
}

#[test]
fn wrong_indentation_vs_orphan_distinction_1() {
    // An Orphan task is:
    // - task that is separated by an empty line from previous elements
    // - AND has any indentation other than zero.
    //
    // Wrong indentation
    // - a sub task / task that is attached to a parent task (no empty line between other element)
    // - AND that has an indentation that does not match the subtask level

    let input = "\
- [ ] testing
  - [ ] subtask -> OK
  - [ ] another subtask -> OK
   - [ ] subtask WRONG INDENT
 - [ ] subtask WRONG INDENT

 - [ ] ORPHAN (also incorrect indent, but message should be orphan)

  - [ ] ORPHAN
";

    let mut issues = check_all(&p(input), &crate::config::Config::default());
    issues.sort_by_key(|i| i.location.line);

    assert_eq!(issues[0].code, ErrorCode::WrongIndentation);
    assert_eq!(issues[1].code, ErrorCode::WrongIndentation);
    assert_eq!(issues[2].code, ErrorCode::OrphanedSubtask);
    assert_eq!(issues[3].code, ErrorCode::OrphanedSubtask);
}

#[test]
fn missing_space_behind_box_vs_wrong_body_indent() {
    let input = "\
- [ ] ok, has space
- [ ]MISSING space
  - [ ] valid subtask
WRONG INDENT tasks description
";

    let mut issues = check_all(&p(input), &crate::config::Config::default());
    issues.sort_by_key(|i| i.location.line);

    assert_eq!(issues[0].code, ErrorCode::MissingSpaceAfterBox);
    assert_eq!(issues[0].location.line, 2);

    assert_eq!(issues[1].code, ErrorCode::WrongBodyIndentation);
    assert_eq!(issues[1].location.line, 4);
}

#[test]
fn check_config_independent_skips_undefined_property_and_assignment() {
    // #undeclared/@undeclared would be flagged by check_all against any
    // config that doesn't declare them (including Config::default()), but
    // check_config_independent must never look at properties/users/groups
    // at all — it's meant for use when there's no config worth trusting.
    let input = "\
- [ ] task #undeclared @undeclared
";
    let issues = check_config_independent(&p(input));
    assert!(
        !issues
            .iter()
            .any(|i| i.code == ErrorCode::UndefinedProperty),
        "check_config_independent must not run undefined_property, got: {issues:?}"
    );
    assert!(
        !issues
            .iter()
            .any(|i| i.code == ErrorCode::UndefinedAssignment),
        "check_config_independent must not run undefined_assignment, got: {issues:?}"
    );
}

#[test]
fn check_config_independent_still_runs_structural_checks() {
    // Structural checks (that don't need config) must still run.
    let input = "\
- [ ] 
";
    let issues = check_config_independent(&p(input));
    assert!(
        issues.iter().any(|i| i.code == ErrorCode::EmptyTitle),
        "check_config_independent should still run structural checks like empty_title, got: {issues:?}"
    );
}

#[test]
fn check_all_equals_check_config_independent_plus_config_dependent_checks() {
    // check_all must be exactly the union of check_config_independent and
    // the four config-dependent rules — no rule silently duplicated or
    // dropped when the two were split apart.
    let input = "\
- [ ] task #undeclared @undeclared
";
    let config = crate::config::Config::default();
    let mut from_check_all = check_all(&p(input), &config);
    let mut from_independent_plus_config_dependent = check_config_independent(&p(input));
    from_independent_plus_config_dependent.extend(undefined_property(&p(input), &config));
    from_independent_plus_config_dependent.extend(undefined_assignment(&p(input), &config));
    from_independent_plus_config_dependent.extend(missing_required_subtasks(&p(input), &config));
    from_independent_plus_config_dependent.extend(unrequired_quoted_subtask(&p(input), &config));

    from_check_all.sort_by_key(|i| (i.location.line, format!("{:?}", i.code)));
    from_independent_plus_config_dependent
        .sort_by_key(|i| (i.location.line, format!("{:?}", i.code)));

    assert_eq!(from_check_all, from_independent_plus_config_dependent);
}

#[test]
fn find_node_by_line_locates_top_level_task() {
    let input = "\
- [ ] first task
  - [ ] a subtask
- [ ] second task
";
    let items = p(input);
    let node = find_node_by_line(&items, 3).expect("expected a node at line 3");
    assert_eq!(node.title(), "second task");
}

#[test]
fn find_node_by_line_locates_nested_subtask() {
    let input = "\
- [ ] first task
  - [ ] a subtask
    - [ ] a nested subtask
";
    let items = p(input);
    let node = find_node_by_line(&items, 3).expect("expected a node at line 3");
    assert_eq!(node.title(), "a nested subtask");
}

#[test]
fn find_node_by_line_returns_none_when_no_node_starts_there() {
    let input = "\
- [ ] only task
";
    let items = p(input);
    assert!(find_node_by_line(&items, 2).is_none());
    assert!(find_node_by_line(&items, 0).is_none());
}
