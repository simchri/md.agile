use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn flags_neighbortasks_property_set_at_top_level() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.frontend-implementation]
neighbortasks = [\"do the back end implementation also!\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    // Top-level tasks have no quoted-sibling syntax at all, so this property
    // can never be satisfied here -- see README.vision.md "Neighbor Tasks".
    let content = "\
- [ ] #frontend-implementation: top level feature
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E020"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("#frontend-implementation"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn does_not_flag_property_with_neighbortasks_on_a_subtask() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.frontend-implementation]
neighbortasks = [\"do the back end implementation also!\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
  - [ ] \"do the back end implementation also!\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E020"), "stdout: {stdout:?}");
}

#[test]
fn top_level_restriction_fires_before_and_instead_of_missing_neighbor_task() {
    // Even though a quoted sibling with the exact right title is present
    // (which would normally satisfy E019's "missing neighbor task" check),
    // a top-level task can't carry this property at all. E020 must fire,
    // and E019 must stay silent -- the two are mutually exclusive for the
    // same task.
    let dir = tempdir().unwrap();
    let config = "\
[Properties.frontend-implementation]
neighbortasks = [\"do the back end implementation also!\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] #frontend-implementation: top level feature
- [ ] \"do the back end implementation also!\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E020"), "stdout: {stdout:?}");
    assert!(!stdout.contains("E019"), "stdout: {stdout:?}");
}
