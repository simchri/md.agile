//! Acceptance tests for a markdown-fenced ```toml code block embedded at the
//! top of a `.agile.md` file (see `sample_1.agile.md:1:1` for the original,
//! ad-hoc example this test was written from, mirroring the config example
//! from README.vision.md "Branch Properties" pasted directly into a task
//! file instead of `mdagile.toml`).
//!
//! Project configuration only ever comes from `mdagile.toml`/`.mdagile.toml`
//! at the project root (see [Configuration](../../../doc/usage/config.md)) —
//! `.agile.md` files are parsed for tasks only. A fenced code block is
//! ordinary markdown prose to the task parser: it isn't read as config, and
//! (as long as its lines don't themselves look like task bullets) it
//! produces no parsing issues of its own.

use crate::helpers::run_check;
use std::fs;
use tempfile::tempdir;

#[test]
fn embedded_toml_fence_in_a_task_file_does_not_act_as_live_config() {
    let dir = tempdir().unwrap();
    // No mdagile.toml at all -- only this fenced block, inside the task
    // file itself, which *looks* like it declares `#review`.
    let file_content = "\
```toml
[Properties.review]
subtasks = [\"document review findings\"]
[Properties.review.passed]
neighbortasks = [\"publish feature\"]
subtasks = [\"review passed subtask\"]
[Properties.review.failed]
neighbortasks = [\"create follow up task for fixes\"]
subtasks = [\"review failed subtask\"]
```

- [ ] a task
  - [ ] some sub task that needs #review
    - [ ] \"document review findings\"
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();

    // The fenced block is not real config, so #review is still undefined --
    // reported at its actual line (13), unaffected by the 10 preceding
    // fence/config-lookalike lines.
    assert!(stdout.contains("E008"), "stdout: {stdout:?}");
    assert!(stdout.contains("#review"), "stdout: {stdout:?}");
    assert!(stdout.contains("a.agile.md:13:"), "stdout: {stdout:?}");

    // Consequently, the quoted subtask isn't required by any *real*
    // property either, so it's flagged as an unrequired quoted subtask.
    assert!(stdout.contains("E011"), "stdout: {stdout:?}");
    assert!(stdout.contains("a.agile.md:14:"), "stdout: {stdout:?}");

    // The fenced block's own lines (1-10) must not produce any issues of
    // their own (e.g. no "wrong indentation"/"wrong body indentation" for
    // the ``` fence markers or the bracketed `[Properties...]` lines) --
    // exactly two issues are reported, both from the real tasks below.
    assert_eq!(
        stdout.matches("error[").count(),
        2,
        "expected exactly E008 and E011, nothing else: {stdout:?}"
    );
}

#[test]
fn the_same_requirements_declared_in_real_mdagile_toml_are_satisfied() {
    // Contrast case: moving the exact same property declarations into the
    // real `mdagile.toml` (instead of a fenced block inside the task file)
    // makes the property defined and its resolved "passed" branch's
    // requirements enforceable, confirming the fenced block in the previous
    // test was inert specifically because it wasn't *actually* config, not
    // because of some unrelated parsing quirk.
    let dir = tempdir().unwrap();
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
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] a task
  - [ ] some sub task that needs #review
    - [ ] \"document review findings\"
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
