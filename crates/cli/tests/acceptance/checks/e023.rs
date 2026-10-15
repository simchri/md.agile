//! Acceptance test derived from `sample_1.md` (originally `sample_1.agile.md`):
//! the "Branch Properties" config example from README.vision.md, written out
//! as the two real project files it's meant to live in — `mdagile.toml` (the
//! ```toml block) and `tasks.agile.md` (the task tree) — rather than pasted
//! together into one `.agile.md` file.
//!
//! `#review` alone is not allowed: declaring `branches` in config (even just
//! one) turns a property into a Branch Property for good, so it must always
//! be written pending (`#review:...`) or resolved to one of its outcomes
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
    // `#review:...` is the correct way to say "still being decided" -- it
    // must not trip E023.
    let dir = tempdir().unwrap();
    write_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:...
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

/// Full matrix of cases for a Branch Property that, unlike
/// [`write_review_config`] above, declares `subtasks` *and* `neighbortasks`
/// at both the base level and on every branch, with one neighbor-task name
/// ("common neighbortask") deliberately repeated across the two sibling
/// branches (`.passed` and `.failed`) to exercise cross-branch dedup.
fn write_full_matrix_review_config(dir: &std::path::Path) {
    let config = "\
[Properties.review]
subtasks = [\"document review findings\"]
neighbortasks = [\"top level common neighbortask\"]
[Properties.review.passed]
neighbortasks = [\"review passed neighbortask\", \"common neighbortask\"]
subtasks = [\"review passed subtask\"]
[Properties.review.failed]
subtasks = [\"review failed subtask\"]
neighbortasks = [\"review failed neighbortask\", \"common neighbortask\"]
";
    fs::write(dir.join("mdagile.toml"), config).unwrap();
}

#[test]
fn plain_form_is_flagged_as_e023_even_with_every_requirement_satisfied() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review
    - [ ] \"document review findings\"
  - [ ] \"top level common neighbortask\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E023"), "stdout: {stdout:?}");
}

#[test]
fn pending_form_is_satisfied_by_only_the_base_propertys_own_requirements() {
    // `#review:...` means "still being decided" -- only the base property's
    // own `subtasks`/`neighbortasks` apply; neither branch's extra
    // requirements are in effect yet.
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:...
    - [ ] \"document review findings\"
  - [ ] \"top level common neighbortask\"
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
fn pending_form_missing_the_base_propertys_own_subtask_is_flagged_e010() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:...
  - [ ] \"top level common neighbortask\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E010"), "stdout: {stdout:?}");
    assert!(!stdout.contains("E023"), "stdout: {stdout:?}");
}

#[test]
fn pending_form_missing_the_base_propertys_own_neighbortask_is_flagged_e019() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:...
    - [ ] \"document review findings\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E019"), "stdout: {stdout:?}");
    assert!(!stdout.contains("E023"), "stdout: {stdout:?}");
}

#[test]
fn resolved_passed_is_satisfied_by_the_base_propertys_and_the_passed_branchs_own_requirements() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:passed
    - [ ] \"document review findings\"
    - [ ] \"review passed subtask\"
  - [ ] \"top level common neighbortask\"
  - [ ] \"review passed neighbortask\"
  - [ ] \"common neighbortask\"
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
fn resolved_passed_missing_the_base_propertys_own_subtask_is_flagged_e010() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:passed
    - [ ] \"review passed subtask\"
  - [ ] \"top level common neighbortask\"
  - [ ] \"review passed neighbortask\"
  - [ ] \"common neighbortask\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E010"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("document review findings"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn resolved_passed_missing_the_passed_branchs_own_subtask_is_flagged_e010() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:passed
    - [ ] \"document review findings\"
  - [ ] \"top level common neighbortask\"
  - [ ] \"review passed neighbortask\"
  - [ ] \"common neighbortask\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E010"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("review passed subtask"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn resolved_passed_missing_the_base_propertys_own_neighbortask_is_flagged_e019() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:passed
    - [ ] \"document review findings\"
    - [ ] \"review passed subtask\"
  - [ ] \"review passed neighbortask\"
  - [ ] \"common neighbortask\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E019"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("top level common neighbortask"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn resolved_passed_missing_the_passed_branchs_own_neighbortask_is_flagged_e019() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:passed
    - [ ] \"document review findings\"
    - [ ] \"review passed subtask\"
  - [ ] \"top level common neighbortask\"
  - [ ] \"common neighbortask\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E019"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("review passed neighbortask"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn resolved_passed_does_not_require_the_failed_branchs_own_requirements() {
    // Cross-branch isolation: only the resolved branch's own
    // subtasks/neighbortasks apply -- the sibling `.failed` branch's items
    // are never required, even though they're declared in the same config.
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:passed
    - [ ] \"document review findings\"
    - [ ] \"review passed subtask\"
  - [ ] \"top level common neighbortask\"
  - [ ] \"review passed neighbortask\"
  - [ ] \"common neighbortask\"
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
fn resolved_failed_is_satisfied_by_the_base_propertys_and_the_failed_branchs_own_requirements() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:failed
    - [ ] \"document review findings\"
    - [ ] \"review failed subtask\"
  - [ ] \"top level common neighbortask\"
  - [ ] \"review failed neighbortask\"
  - [ ] \"common neighbortask\"
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
fn resolved_failed_missing_the_failed_branchs_own_subtask_is_flagged_e010() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:failed
    - [ ] \"document review findings\"
  - [ ] \"top level common neighbortask\"
  - [ ] \"review failed neighbortask\"
  - [ ] \"common neighbortask\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E010"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("review failed subtask"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn resolved_failed_missing_the_failed_branchs_own_neighbortask_is_flagged_e019() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:failed
    - [ ] \"document review findings\"
    - [ ] \"review failed subtask\"
  - [ ] \"top level common neighbortask\"
  - [ ] \"common neighbortask\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E019"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("review failed neighbortask"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn resolved_failed_does_not_require_the_passed_branchs_own_requirements() {
    // Cross-branch isolation, symmetric to the `.passed` case above.
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:failed
    - [ ] \"document review findings\"
    - [ ] \"review failed subtask\"
  - [ ] \"top level common neighbortask\"
  - [ ] \"review failed neighbortask\"
  - [ ] \"common neighbortask\"
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
fn resolved_to_an_undefined_branch_outcome_is_flagged_e022() {
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:unknown
    - [ ] \"document review findings\"
  - [ ] \"top level common neighbortask\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E022"), "stdout: {stdout:?}");
}

#[test]
fn a_neighbortask_name_shared_by_two_sibling_branches_satisfies_both_resolved_outcomes_at_once() {
    // ".passed" and ".failed" both declare "common neighbortask". Two
    // sibling tasks -- one resolved to each outcome -- both require it, but
    // a single shared quoted sibling satisfies both: required-neighbor
    // matching is presence-based among the sibling list, not consumed one
    // requirer at a time, so this is not double-counted.
    let dir = tempdir().unwrap();
    write_full_matrix_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review:passed
    - [ ] \"document review findings\"
    - [ ] \"review passed subtask\"
  - [ ] another sub task that needs #review:failed
    - [ ] \"document review findings\"
    - [ ] \"review failed subtask\"
  - [ ] \"top level common neighbortask\"
  - [ ] \"review passed neighbortask\"
  - [ ] \"review failed neighbortask\"
  - [ ] \"common neighbortask\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
