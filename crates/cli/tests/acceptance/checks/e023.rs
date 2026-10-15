//! Acceptance test derived from `sample_1.md` (originally `sample_1.agile.md`):
//! the "Branch Properties" config example from README.vision.md, written out
//! as the two real project files it's meant to live in — `mdagile.toml` (the
//! ```toml block) and `tasks.agile.md` (the task tree) — rather than pasted
//! together into one `.agile.md` file.
//!
//! `#review` alone is not allowed: declaring `branches` in config (even just
//! one) turns a property into a Branch Property for good, so it must always
//! be written pending (`#review...`) or resolved to one of its outcomes
//! (`#review:passed`/`#review:failed`) — never used in its plain form. Using
//! the plain form is flagged as E023 "Plain Branching Property".

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

fn write_review_config(dir: &std::path::Path) {
    let config = "\
[Properties.review]
subtasks = [\"document review findings\"]
[Properties.review.passed]
neighbortasks = [\"publish feature\"]
subtasks = [\"review passed subtask\"]
[Properties.review.failed]
neighbortasks = [\"create follow up task for fixes\"]
subtasks = [\"review failed subtask\"]
";
    fs::write(dir.join("mdagile.toml"), config).unwrap();
}

#[test]
fn plain_property_usage_is_flagged_even_with_its_required_subtask_present() {
    let dir = tempdir().unwrap();
    write_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review
    - [ ] \"document review findings\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E023"), "stdout: {stdout:?}");
    assert!(stdout.contains("#review"), "stdout: {stdout:?}");
}

#[test]
fn plain_property_usage_still_requires_the_base_propertys_own_subtask() {
    // Dropping the one required subtask still trips E010 alongside E023 --
    // the base property's own requirements are enforced regardless of
    // whether the property also happens to declare branches.
    let dir = tempdir().unwrap();
    write_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E010"), "stdout: {stdout:?}");
    assert!(stdout.contains("E023"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("document review findings"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn plain_property_usage_is_flagged_regardless_of_done_status() {
    let dir = tempdir().unwrap();
    write_review_config(dir.path());
    let file_content = "\
- [x] a task
  - [x] some sub task that needs #review
    - [x] \"document review findings\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E023"), "stdout: {stdout:?}");
}

#[test]
fn pending_property_usage_is_not_flagged() {
    // `#review...` is the correct way to say "still being decided" -- it
    // must not trip E023.
    let dir = tempdir().unwrap();
    write_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review...
    - [ ] \"document review findings\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn resolved_property_usage_is_not_flagged_as_plain_branching_property() {
    // `#review:passed` is the correct way to resolve the outcome, so it must
    // not be flagged as E023 -- only the base property's own subtask is
    // exercised here; its resolved branch's own extra requirements are
    // already covered by the dedicated Branch Properties edge-case tests
    // in `checks/e010.rs`/`checks/e012.rs`/`checks/e019.rs`/`checks/e021.rs`.
    let dir = tempdir().unwrap();
    write_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:passed
    - [ ] \"document review findings\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("E023"), "stdout: {stdout:?}");
}
