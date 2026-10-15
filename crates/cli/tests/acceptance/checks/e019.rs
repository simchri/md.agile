use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

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
fn flags_neighbortasks_property_set_at_top_level_as_always_missing() {
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
- [ ] \"do the back end implementation also!\"
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E019"), "stdout: {stdout:?}");
}
