//! Acceptance test derived from `sample_1.md` (originally `sample_1.agile.md`):
//! the "Branch Properties" config example from README.vision.md, written out
//! as the two real project files it's meant to live in — `mdagile.toml` (the
//! ```toml block) and `tasks.agile.md` (the task tree) — rather than pasted
//! together into one `.agile.md` file.
//!
//! The task here uses the property in its plain form (`#review`, neither the
//! pending `#review...` nor a resolved `#review:passed`/`#review:failed`),
//! even though `review` declares branches in config. This only requires the
//! base property's own `subtasks`/`neighbortasks` ("document review
//! findings") — it's unrelated to, and doesn't trigger, the Branch
//! Properties pending/resolved machinery (E021/E022) at all, since those
//! only apply once a task actually carries the `...`/`:branch` syntax.

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
fn plain_property_usage_with_its_required_subtask_present_passes_check() {
    let dir = tempdir().unwrap();
    write_review_config(dir.path());
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review
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
fn plain_property_usage_still_requires_the_base_propertys_own_subtask() {
    // Dropping the one required subtask still trips E010, exactly as it
    // would for any ordinary (non-branching) property -- the base
    // property's own requirements are enforced regardless of whether the
    // property also happens to declare branches.
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
    assert!(
        stdout.contains("document review findings"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn plain_property_usage_does_not_enforce_any_branchs_own_requirements() {
    // Neither branch's own extra requirements ("publish feature"/"create
    // follow up task for fixes" neighbor tasks, or their own subtasks) are
    // enforced while the property is used in its plain form -- those only
    // apply once the property actually resolves to that branch
    // (`#review:passed`/`#review:failed`), not merely by declaring them in
    // config. This is not flagged as E021 ("Unresolved Branch Property")
    // either, since that check only fires for the pending `#review...` form.
    let dir = tempdir().unwrap();
    write_review_config(dir.path());
    let file_content = "\
- [x] a task
  - [x] some sub task that needs #review
    - [x] \"document review findings\"
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
