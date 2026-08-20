//! Acceptance-level tests for the "mark eligible tasks bold" feature
//! (`agile task next`): the first `Todo` leaf in document order is
//! highlighted with ANSI bold escapes, subject to `--mine`/`--as` identity
//! eligibility. These exercise the full CLI — including `mdagile.toml`
//! config loading (`[Properties.*]`, `[Users.*]`) and git identity
//! resolution — rather than calling the rendering helpers directly.

use crate::helpers::run_agile;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git command failed to start");
    assert!(status.success(), "git {args:?} failed");
}

fn stdout_of(dir: &std::path::Path, args: &[&str]) -> String {
    let out = run_agile(dir, args);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn bolds_the_only_todo_leaf() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next"]);
    assert_eq!(stdout, format!("{BOLD}[ ] parent task{RESET}\n"));
}

#[test]
fn bolds_first_todo_leaf_in_document_order() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [x] already done subtask
  - [ ] first todo leaf
  - [ ] second todo leaf
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next"]);
    assert_eq!(
        stdout,
        format!(
            "[ ] parent task\n  [x] already done subtask\n  {BOLD}[ ] first todo leaf{RESET}\n  [ ] second todo leaf\n"
        )
    );
}

#[test]
fn skips_non_leaf_todo_nodes() {
    // A `Todo` node with children is not itself a leaf, so it is never
    // bolded - only the actual leaf (a node with no children) is.
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] mid-level subtask with children
    - [ ] actual leaf
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next"]);
    assert_eq!(
        stdout,
        format!(
            "[ ] parent task\n  [ ] mid-level subtask with children\n    {BOLD}[ ] actual leaf{RESET}\n"
        )
    );
}

#[test]
fn bolds_nothing_when_no_todo_leaf_exists() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] parent task
  - [x] done subtask
  - [-] cancelled subtask
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next"]);
    assert!(stdout.is_empty(), "stdout: {stdout:?}");
}

#[test]
fn bolds_within_dotted_addressed_subtask_subtree() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] addressed subtask
    - [x] already done grandchild
    - [ ] next leaf
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next", "1.1"]);
    assert_eq!(
        stdout,
        format!(
            "[ ] addressed subtask\n  [x] already done grandchild\n  {BOLD}[ ] next leaf{RESET}\n"
        )
    );
}

#[test]
fn includes_body_when_full_flag_given() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  some body text
  - [ ] leaf with a body
    leaf body line
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next", "--full"]);
    assert_eq!(
        stdout,
        format!(
            "[ ] parent task\n  some body text\n  {BOLD}[ ] leaf with a body{RESET}\n    leaf body line\n"
        )
    );
}

#[test]
fn omits_body_by_default() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  some body text
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next"]);
    assert_eq!(stdout, format!("{BOLD}[ ] parent task{RESET}\n"));
}

#[test]
fn mine_skips_leaf_assigned_to_someone_else() {
    // Regression test for the reported bug: bolding must respect eligibility
    // for the resolved git identity, not just "first Todo leaf" unconditionally.
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] leaf assigned to bob @bob
  - [ ] leaf eligible for alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next", "--mine"]);
    assert_eq!(
        stdout,
        format!(
            "[ ] parent task\n  [ ] leaf assigned to bob @bob\n  {BOLD}[ ] leaf eligible for alice{RESET}\n"
        )
    );
}

#[test]
fn mine_bolds_nothing_when_no_leaf_eligible_for_identity() {
    // The top-level task itself is filtered out by eligibility (it has no
    // Todo child eligible for alice), so `agile task next --mine` finds no
    // matching top-level task at all and prints nothing.
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] leaf assigned to bob @bob
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next", "--mine"]);
    assert!(stdout.is_empty(), "stdout: {stdout:?}");
}

#[test]
fn mine_skips_ordered_leaf_eligible_for_identity_but_blocked_by_incomplete_lower_order_sibling_assigned_to_someone_else()
 {
    // Regression test: ordered subtasks (`1. ...`, `2. ...`) establish an
    // execution sequence among siblings — a higher-ordered sibling cannot
    // actually be worked on while a lower-ordered one is still incomplete
    // (see E015 / `invalid_order::blocked_by_incomplete_lower_order`).
    // Eligibility must take this into account: alice is nominally assigned
    // "2. second step", but bob's still-incomplete "1. first step" blocks
    // it, so there is nothing alice can actually do yet, and `--mine`
    // should bold nothing (not the blocked step assigned to her).
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] 1. first step @bob
  - [ ] 2. second step @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next", "--mine"]);
    assert!(stdout.is_empty(), "stdout: {stdout:?}");
    assert!(!stdout.contains(BOLD));
}

#[test]
fn mine_bolds_ordered_leaf_once_blocking_lower_order_sibling_is_done() {
    // Same setup as above, but bob's "1. first step" is now done, so
    // alice's "2. second step" is actually actionable and should be bolded.
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] parent task
  - [x] 1. first step @bob
  - [ ] 2. second step @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next", "--mine"]);
    assert_eq!(
        stdout,
        format!("[ ] parent task\n  [x] first step @bob\n  {BOLD}[ ] second step @alice{RESET}\n")
    );
}

#[test]
fn without_identity_still_bolds_blocked_ordered_leaf_unconditionally() {
    // Without `--mine`/`--as`, the unconditional "first Todo leaf" behavior
    // is unchanged: order-blocking is an eligibility-only concern, not a
    // general "is this leaf actionable" concern for the unconditional case.
    // (`agile task done` still separately rejects completing an
    // out-of-order leaf; this is purely about what gets highlighted.)
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] 1. first step @bob
  - [ ] 2. second step @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next"]);
    assert_eq!(
        stdout,
        format!("[ ] parent task\n  {BOLD}[ ] first step @bob{RESET}\n  [ ] second step @alice\n")
    );
}

#[test]
fn bolds_unconditionally_without_mine_or_as() {
    // Without `--mine`/`--as`, the old unconditional "first Todo leaf"
    // behavior still applies even to a leaf assigned to someone else.
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] leaf assigned to bob @bob
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next"]);
    assert_eq!(
        stdout,
        format!("[ ] parent task\n  {BOLD}[ ] leaf assigned to bob @bob{RESET}\n")
    );
}

/// A VER-161-style task: quoted subtask titles required by a
/// `[Properties.systemtest]` config entry (with `subtasks_allow_cancel`),
/// and one subtask assigned via `@Gini`.
const VER_161_FILE_CONTENT: &str = "\
- [ ] VER-161 System Test Image Installation and Smoke Test #systemtest
  - [x] \"1. write draft\"
  - [ ] \"2. assign review\" @Gini
  - [ ] \"3. implement feedback\"
  - [ ] \"4. approved\"
";

const VER_161_CONFIG: &str = "\
[Properties.systemtest]
subtasks = [\"1. write draft\", \"2. assign review\", \"3. implement feedback\", \"4. approved\"]
subtasks_allow_cancel = [true, true, true, true]

[Users.Gini]
git_emails = [\"gini@example.com\"]

[Users.alice]
git_emails = [\"alice@example.com\"]
";

#[test]
fn bolds_first_todo_leaf_among_quoted_property_subtasks() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("mdagile.toml"), VER_161_CONFIG).unwrap();
    fs::write(dir.path().join("tasks.agile.md"), VER_161_FILE_CONTENT).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next"]);
    assert_eq!(
        stdout,
        format!(
            "[ ] VER-161 System Test Image Installation and Smoke Test #systemtest\n  [x] 1. write draft\n  {BOLD}[ ] 2. assign review{RESET}\n  [ ] 3. implement feedback\n  [ ] 4. approved\n"
        )
    );
}

#[test]
fn as_gini_bolds_leaf_assigned_to_gini() {
    // With `--as Gini`, the leaf assigned to her (via `@Gini`) is eligible
    // and gets bolded, same as the unconditional case.
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    fs::write(dir.path().join("mdagile.toml"), VER_161_CONFIG).unwrap();
    fs::write(dir.path().join("tasks.agile.md"), VER_161_FILE_CONTENT).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next", "--as", "Gini"]);
    assert_eq!(
        stdout,
        format!(
            "[ ] VER-161 System Test Image Installation and Smoke Test #systemtest\n  [x] 1. write draft\n  {BOLD}[ ] 2. assign review{RESET}\n  [ ] 3. implement feedback\n  [ ] 4. approved\n"
        )
    );
}

#[test]
fn as_alice_skips_leaf_assigned_to_gini() {
    // For an identity other than Gini, the leaf assigned to her ("2. assign
    // review") is not eligible. Since these subtasks are strictly ordered
    // (the quoted "N. ..." titles carry real order numbers, same as
    // unquoted ordered subtasks), the later "3. implement feedback" is also
    // blocked while "2." remains incomplete, even though it's unassigned —
    // alice can't jump ahead of an incomplete lower-ordered sibling. So
    // nothing is eligible for her, and `--as alice` bolds nothing.
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    fs::write(dir.path().join("mdagile.toml"), VER_161_CONFIG).unwrap();
    fs::write(dir.path().join("tasks.agile.md"), VER_161_FILE_CONTENT).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next", "--as", "alice"]);
    assert_eq!(stdout, "");
}

#[test]
fn mine_does_not_bold_an_order_blocked_leaf_assigned_to_identity_even_when_a_later_unordered_sibling_is_the_real_eligible_one()
 {
    // Regression test for a second angle on the same order-blocking bug:
    // the *task-level* eligibility check (whether to show the task at all
    // under `--mine`) already excludes order-blocked children, via
    // `rules::is_eligible_for` recursing over `task.children`. But the
    // *leaf-bolding* walk (which finds the specific line to highlight)
    // previously called `rules::is_eligible_for` directly on each candidate
    // leaf in isolation — which, for a leaf with no children of its own,
    // only checks its own assignment markers and never re-derives whether
    // *that specific leaf* is order-blocked by its siblings.
    //
    // So here: "2. blocked step" is nominally assigned to alice but blocked
    // by bob's incomplete "1. first step". The task is still eligible for
    // alice overall (because "separate unordered step", also hers, is
    // unblocked and unordered). But document order visits "2. blocked step"
    // before "separate unordered step" — so the buggy leaf-level check
    // would bold "2. blocked step" (wrong: not actually actionable) instead
    // of skipping over it to bold "separate unordered step" (the real next
    // actionable line for alice).
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]

[Users.bob]
git_emails = [\"bob@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] 1. first step @bob
  - [ ] 2. blocked step @alice
  - [ ] separate unordered step @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next", "--mine"]);
    assert_eq!(
        stdout,
        format!(
            "[ ] parent task\n  [ ] first step @bob\n  [ ] blocked step @alice\n  {BOLD}[ ] separate unordered step @alice{RESET}\n"
        )
    );
}
