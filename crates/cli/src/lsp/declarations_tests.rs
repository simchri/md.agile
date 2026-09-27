use super::*;

#[test]
fn indexes_dotted_and_flat_declarations_at_source_lines() {
    let file_content = "\
[Properties.feature] # description
brief = \"[Properties.fake]\"
[Properties]
\"fix bug\" = {}
[Users.alice]
[Groups.team]
";
    let index = DeclarationIndex::parse(file_content).unwrap();
    assert_eq!(index.line(Kind::Property, "feature"), Some(0));
    assert_eq!(index.line(Kind::Property, "fix bug"), Some(3));
    assert_eq!(index.line(Kind::Property, "fake"), None);
    assert_eq!(index.line(Kind::User, "alice"), Some(4));
    assert_eq!(index.line(Kind::Group, "team"), Some(5));
    assert_eq!(index.names(&[Kind::Property]), vec!["feature", "fix bug"]);
}

#[test]
fn handles_comments_multiline_strings_quoted_keys_and_unicode() {
    let file_content = "\
# [Properties.fake]
[General]
note = \"\"\"text
[Properties.fake]
\"\"\"
[Properties.\"café\"] # actual header
[Users]
\"a.b\" = {}
";
    let index = DeclarationIndex::parse(file_content).unwrap();
    assert_eq!(index.line(Kind::Property, "café"), Some(5));
    assert_eq!(index.line(Kind::User, "a.b"), Some(7));
    assert_eq!(index.line(Kind::Property, "fake"), None);
}

#[test]
fn assignment_collision_uses_first_source_occurrence() {
    let file_content = "\
[Groups.sam]
[Users.sam]
";
    let index = DeclarationIndex::parse(file_content).unwrap();
    assert_eq!(index.assignment_line("sam"), Some(0));
    assert_eq!(index.names(&[Kind::User, Kind::Group]), vec!["sam"]);
}

#[test]
fn malformed_toml_is_an_error_not_a_partial_index() {
    let file_content = "\
[Properties.feature
";
    assert!(DeclarationIndex::parse(file_content).is_err());
}

#[test]
fn indexes_inline_sections_without_matching_nested_values() {
    let file_content = "\
Properties = { feature = { brief = \"hello\" } }
Users = { alice = { git_names = [\"Alice\"] } }
";
    let index = DeclarationIndex::parse(file_content).unwrap();
    assert_eq!(index.line(Kind::Property, "feature"), Some(0));
    assert_eq!(index.line(Kind::User, "alice"), Some(1));
    assert_eq!(index.line(Kind::Property, "brief"), None);
}
