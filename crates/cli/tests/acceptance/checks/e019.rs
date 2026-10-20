use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn resolved_branch_satisfies_a_pending_required_neighbor() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.feature]
neighbortasks = [\"check #review:...\"]
[Properties.review]
[Properties.review.passed]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] parent
  - [ ] implement #feature
  - [ ] \"check #review:passed\"
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}, stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn flags_missing_neighbor_task() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.frontend-implementation]
neighbortasks = [\"do the back end implementation also!\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E019"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("do the back end implementation also!"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn does_not_flag_task_with_neighbor_task_present() {
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

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn does_not_flag_a_top_level_property_as_missing_neighbor_task() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.frontend-implementation]
neighbortasks = [\"do the back end implementation also!\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    // A property requiring neighbor tasks set at the top level is reported
    // as E020 "Neighbor Task On Top Level" instead -- see e020.rs -- so
    // E019 should stay silent about it here.
    let content = "\
- [ ] #frontend-implementation: top level feature
- [ ] \"do the back end implementation also!\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E019"), "stdout: {stdout:?}");
}

#[test]
fn a_resolved_branchs_required_neighbor_identical_to_the_base_propertys_own_is_satisfied_once() {
    let dir = tempdir().unwrap();
    let config = "\
[Properties.review]
neighbortasks = [\"sign off meeting\"]
[Properties.review.passed]
neighbortasks = [\"sign off meeting\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let content = "\
- [ ] build something
  - [ ] perform #review:passed
  - [ ] \"sign off meeting\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E019"), "stdout: {stdout:?}");
}
