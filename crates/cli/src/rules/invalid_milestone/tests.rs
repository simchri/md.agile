use crate::parser::parse;
use crate::rules::ErrorCode;
use std::path::PathBuf;

fn p(input: &str) -> Vec<crate::parser::FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

#[test]
fn allows_unique_milestone_names() {
    let file_content = "\
- [x] task one
#MILESTONE: Alpha
- [ ] task two
#MILESTONE: Beta
- [ ] task three
";
    let issues = super::invalid_milestone(&p(file_content));

    assert_eq!(issues, vec![]);
}

#[test]
fn flags_duplicate_milestone_name_within_a_single_file() {
    let file_content = "\
- [x] task one
#MILESTONE: Alpha
- [ ] task two
#MILESTONE: Alpha
- [ ] task three
";
    let issues = super::invalid_milestone(&p(file_content));

    assert_eq!(
        issues.len(),
        2,
        "both same-named milestones are flagged, not just the second"
    );
    assert!(
        issues
            .iter()
            .all(|i| i.code == ErrorCode::DuplicateMilestoneName)
    );
    assert_eq!(issues[0].location.line, 2);
    assert_eq!(issues[1].location.line, 4);
}

#[test]
fn flags_duplicate_milestone_name_across_files() {
    // `parse_files` concatenates every project file's items into one flat
    // `Vec<FileItem>`, so duplicate detection across files reduces to the
    // exact same scan as the single-file case above.
    let mut items = p("\
#MILESTONE: Release of MVP
- [ ] task in file a
");
    items.extend(parse(
        "\
#MILESTONE: Release of MVP
- [ ] task in file b
",
        PathBuf::from("b.agile.md"),
    ));

    let issues = super::invalid_milestone(&items);

    assert_eq!(issues.len(), 2);
    assert!(
        issues
            .iter()
            .all(|i| i.code == ErrorCode::DuplicateMilestoneName)
    );
    // Sorted by (path, line) -- "b.agile.md" sorts before "test.agile.md".
    assert_eq!(issues[0].location.path, PathBuf::from("b.agile.md"));
    assert_eq!(issues[1].location.path, PathBuf::from("test.agile.md"));
}

#[test]
fn does_not_flag_distinct_milestone_names() {
    let file_content = "\
- [x] task one
#MILESTONE: Alpha
- [ ] task two
#MILESTONE: Beta
- [ ] task three
#MILESTONE: Gamma
";
    let issues = super::invalid_milestone(&p(file_content));

    assert_eq!(issues, vec![]);
}

#[test]
fn flags_bare_milestone_tag_with_no_name() {
    let file_content = "\
- [x] task one
#MILESTONE
- [ ] task two
";
    let issues = super::invalid_milestone(&p(file_content));

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::MissingMilestoneName);
    assert_eq!(issues[0].location.line, 2);
}

#[test]
fn flags_milestone_tag_with_colon_but_no_name() {
    let file_content = "\
#MILESTONE:
- [ ] task
";
    let issues = super::invalid_milestone(&p(file_content));

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::MissingMilestoneName);
}

#[test]
fn nameless_milestones_are_not_also_flagged_as_duplicates_of_each_other() {
    let file_content = "\
#MILESTONE
- [ ] task one
#MILESTONE:
- [ ] task two
";
    let issues = super::invalid_milestone(&p(file_content));

    assert_eq!(issues.len(), 2);
    assert!(
        issues
            .iter()
            .all(|i| i.code == ErrorCode::MissingMilestoneName),
        "expected only E018s, got: {issues:?}"
    );
}
