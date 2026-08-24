//! Acceptance-level tests for task eligibility (`agile task next`): the
//! first `Todo` leaf in document order that is eligible for the resolved
//! `--mine`/`--as` identity is marked as the concrete "next" actionable
//! line. These exercise the full CLI — including `mdagile.toml` config
//! loading (`[Properties.*]`, `[Users.*]`) and git identity resolution —
//! rather than calling the rendering helpers directly.
//!
//! Most tests here use `--no-markup` so the "next" line is marked with a
//! plain-text `" <=="` suffix instead of ANSI bold escapes — this is an
//! eligibility concern, not a display-detail concern, and `--no-markup`
//! keeps the expected strings simple. The default (bolded) rendering is a
//! pure display detail and is covered by a single dedicated test below
//! (`bolds_the_next_eligible_line_by_default`).

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
fn marks_the_only_todo_leaf() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);

    // Assert
    let expected = "\
1 [ ] parent task <==
";
    assert_eq!(stdout, expected);
}

#[test]
fn marks_first_todo_leaf_in_document_order() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [x] already done subtask
  - [ ] first todo leaf
  - [ ] second todo leaf
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] parent task
1.1   [x] already done subtask
1.2   [ ] first todo leaf <==
1.3   [ ] second todo leaf
";
    assert_eq!(stdout, expected);
}

#[test]
fn skips_non_leaf_todo_nodes() {
    // A `Todo` node with children is not itself a leaf, so it is never
    // marked - only the actual leaf (a node with no children) is.

    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] mid-level subtask with children
    - [ ] actual leaf
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);

    // Assert
    let expected = "\
1     [ ] parent task
1.1     [ ] mid-level subtask with children
1.1.1     [ ] actual leaf <==
";
    assert_eq!(stdout, expected);
}

#[test]
fn marks_a_todo_node_whose_children_are_all_resolved_even_though_it_has_children() {
    // A `Todo` node with children is not skipped when none of those
    // children remain actionable (all done/cancelled) - there's nothing
    // left below it to delegate to, so it becomes the next task itself,
    // same as a literal leaf would. Eligibility is not restricted to
    // literal leaves - see the module doc on `rules::next_task`.

    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task with only resolved children
  - [x] done subtask
  - [-] cancelled subtask
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] parent task with only resolved children <==
1.1   [x] done subtask
1.2   [-] cancelled subtask
";
    assert_eq!(stdout, expected);
}

#[test]
fn mine_selects_the_same_top_level_task_as_without_mine_when_its_children_are_all_resolved() {
    // Regression test for the reported bug: `agile task next --mine` used
    // to silently skip past a `Todo` top-level task whose children were all
    // already done/cancelled (eligibility was leaf-only, so a non-leaf
    // `Todo` node with no remaining actionable child was never considered
    // eligible), instead surfacing a *different*, later top-level task -
    // even though plain `agile task next` (no `--mine`) correctly picked
    // the first one, since it only checks the top-level task's own status.
    // Both must now agree: eligibility applies uniformly at every level,
    // not just to literal leaves.

    // Arrange
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "alice@example.com"]);
    git(dir.path(), &["config", "user.name", "Alice"]);

    let config = "\
[Users.alice]
git_emails = [\"alice@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    let file_content = "\
- [ ] first task, all children already resolved
  - [x] done subtask
  - [-] cancelled subtask
- [ ] second task, still has an actionable leaf
  - [ ] an actual leaf
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let without_mine = stdout_of(dir.path(), &["task", "next", "--no-markup"]);
    let with_mine = stdout_of(dir.path(), &["task", "next", "--mine", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] first task, all children already resolved <==
1.1   [x] done subtask
1.2   [-] cancelled subtask
";
    assert_eq!(without_mine, expected);
    assert_eq!(with_mine, expected);
}

#[test]
fn marks_nothing_when_no_todo_leaf_exists() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] parent task
  - [x] done subtask
  - [-] cancelled subtask
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);

    // Assert
    assert!(stdout.is_empty(), "stdout: {stdout:?}");
}

#[test]
fn marks_within_dotted_addressed_subtask_subtree() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] addressed subtask
    - [x] already done grandchild
    - [ ] next leaf
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "1.1", "--no-markup"]);

    // Assert
    let expected = "\
1.1   [ ] addressed subtask
1.1.1   [x] already done grandchild
1.1.2   [ ] next leaf <==
";
    assert_eq!(stdout, expected);
}

#[test]
fn includes_body_when_full_flag_given() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  some body text
  - [ ] leaf with a body
    leaf body line
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--full", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] parent task
      some body text
1.1   [ ] leaf with a body <==
        leaf body line
";
    assert_eq!(stdout, expected);
}

#[test]
fn omits_body_by_default() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  some body text
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);

    // Assert
    let expected = "\
1 [ ] parent task <==
";
    assert_eq!(stdout, expected);
}

#[test]
fn mine_skips_leaf_assigned_to_someone_else() {
    // Regression test for the reported bug: marking must respect eligibility
    // for the resolved git identity, not just "first Todo leaf" unconditionally.

    // Arrange
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

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--mine", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] parent task
1.1   [ ] leaf assigned to bob @bob
1.2   [ ] leaf eligible for alice <==
";
    assert_eq!(stdout, expected);
}

#[test]
fn mine_marks_nothing_when_no_leaf_eligible_for_identity() {
    // The top-level task itself is filtered out by eligibility (it has no
    // Todo child eligible for alice), so `agile task next --mine` finds no
    // matching top-level task at all and prints nothing.

    // Arrange
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

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--mine", "--no-markup"]);

    // Assert
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
    // should mark nothing (not the blocked step assigned to her).

    // Arrange
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

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--mine", "--no-markup"]);

    // Assert
    assert!(stdout.is_empty(), "stdout: {stdout:?}");
    assert!(!stdout.contains("<=="));
}

#[test]
fn mine_marks_ordered_leaf_once_blocking_lower_order_sibling_is_done() {
    // Same setup as above, but bob's "1. first step" is now done, so
    // alice's "2. second step" is actually actionable and should be marked.

    // Arrange
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

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--mine", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] parent task
1.1   [x] first step @bob
1.2   [ ] second step @alice <==
";
    assert_eq!(stdout, expected);
}

#[test]
fn without_identity_still_marks_blocked_ordered_leaf_unconditionally() {
    // Without `--mine`/`--as`, the unconditional "first Todo leaf" behavior
    // is unchanged: order-blocking is an eligibility-only concern, not a
    // general "is this leaf actionable" concern for the unconditional case.
    // (`agile task done` still separately rejects completing an
    // out-of-order leaf; this is purely about what gets highlighted.)

    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] 1. first step @bob
  - [ ] 2. second step @alice
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] parent task
1.1   [ ] first step @bob <==
1.2   [ ] second step @alice
";
    assert_eq!(stdout, expected);
}

#[test]
fn marks_unconditionally_without_mine_or_as() {
    // Without `--mine`/`--as`, the old unconditional "first Todo leaf"
    // behavior still applies even to a leaf assigned to someone else.

    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [ ] leaf assigned to bob @bob
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] parent task
1.1   [ ] leaf assigned to bob @bob <==
";
    assert_eq!(stdout, expected);
}

/// A VER-161-style task: quoted subtask titles required by a
/// `[Properties.systemtest]` config entry (with `subtasks_allow_cancel`),
/// and one subtask assigned via `@Gini`.
#[test]
fn marks_first_todo_leaf_among_quoted_property_subtasks() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] VER-161 System Test Image Installation and Smoke Test #systemtest
  - [x] \"1. write draft\"
  - [ ] \"2. assign review\" @Gini
  - [ ] \"3. implement feedback\"
  - [ ] \"4. approved\"
";
    let config = "\
[Properties.systemtest]
subtasks = [\"1. write draft\", \"2. assign review\", \"3. implement feedback\", \"4. approved\"]
subtasks_allow_cancel = [true, true, true, true]

[Users.Gini]
git_emails = [\"gini@example.com\"]

[Users.alice]
git_emails = [\"alice@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] VER-161 System Test Image Installation and Smoke Test #systemtest
1.1   [x] 1. write draft
1.2   [ ] 2. assign review @Gini <==
1.3   [ ] 3. implement feedback
1.4   [ ] 4. approved
";
    assert_eq!(stdout, expected);
}

#[test]
fn as_gini_marks_leaf_assigned_to_gini() {
    // With `--as Gini`, the leaf assigned to her (via `@Gini`) is eligible
    // and gets marked, same as the unconditional case.

    // Arrange
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    let file_content = "\
- [ ] VER-161 System Test Image Installation and Smoke Test #systemtest
  - [x] \"1. write draft\"
  - [ ] \"2. assign review\" @Gini
  - [ ] \"3. implement feedback\"
  - [ ] \"4. approved\"
";
    let config = "\
[Properties.systemtest]
subtasks = [\"1. write draft\", \"2. assign review\", \"3. implement feedback\", \"4. approved\"]
subtasks_allow_cancel = [true, true, true, true]

[Users.Gini]
git_emails = [\"gini@example.com\"]

[Users.alice]
git_emails = [\"alice@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--as", "Gini", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] VER-161 System Test Image Installation and Smoke Test #systemtest
1.1   [x] 1. write draft
1.2   [ ] 2. assign review @Gini <==
1.3   [ ] 3. implement feedback
1.4   [ ] 4. approved
";
    assert_eq!(stdout, expected);
}

#[test]
fn as_alice_skips_leaf_assigned_to_gini() {
    // For an identity other than Gini, the leaf assigned to her ("2. assign
    // review") is not eligible. Since these subtasks are strictly ordered
    // (the quoted "N. ..." titles carry real order numbers, same as
    // unquoted ordered subtasks), the later "3. implement feedback" is also
    // blocked while "2." remains incomplete, even though it's unassigned —
    // alice can't jump ahead of an incomplete lower-ordered sibling. So
    // nothing is eligible for her, and `--as alice` marks nothing.

    // Arrange
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    let file_content = "\
- [ ] VER-161 System Test Image Installation and Smoke Test #systemtest
  - [x] \"1. write draft\"
  - [ ] \"2. assign review\" @Gini
  - [ ] \"3. implement feedback\"
  - [ ] \"4. approved\"
";
    let config = "\
[Properties.systemtest]
subtasks = [\"1. write draft\", \"2. assign review\", \"3. implement feedback\", \"4. approved\"]
subtasks_allow_cancel = [true, true, true, true]

[Users.Gini]
git_emails = [\"gini@example.com\"]

[Users.alice]
git_emails = [\"alice@example.com\"]
";
    fs::write(dir.path().join("mdagile.toml"), config).unwrap();
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(
        dir.path(),
        &["task", "next", "--as", "alice", "--no-markup"],
    );

    // Assert
    assert_eq!(stdout, "");
}

#[test]
fn mine_does_not_mark_an_order_blocked_leaf_assigned_to_identity_even_when_a_later_unordered_sibling_is_the_real_eligible_one()
 {
    // Regression test for a second angle on the same order-blocking bug:
    // the *task-level* eligibility check (whether to show the task at all
    // under `--mine`) already excludes order-blocked children, via
    // `rules::is_eligible_for` recursing over `task.children`. But the
    // *leaf-marking* walk (which finds the specific line to highlight)
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
    // would mark "2. blocked step" (wrong: not actually actionable) instead
    // of skipping over it to mark "separate unordered step" (the real next
    // actionable line for alice).

    // Arrange
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

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next", "--mine", "--no-markup"]);

    // Assert
    let expected = "\
1   [ ] parent task
1.1   [ ] first step @bob
1.2   [ ] blocked step @alice
1.3   [ ] separate unordered step @alice <==
";
    assert_eq!(stdout, expected);
}

/// The default (no `--no-markup`) rendering is a pure display detail —
/// bolding is exercised once here, not repeated for every eligibility
/// scenario above (all of which use `--no-markup` for simplicity).
#[test]
fn bolds_the_next_eligible_line_by_default() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] parent task
  - [x] already done subtask
  - [ ] first todo leaf
  - [ ] second todo leaf
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let stdout = stdout_of(dir.path(), &["task", "next"]);

    // Assert
    let expected = format!(
        "\
1   [ ] parent task
1.1   [x] already done subtask
1.2   {BOLD}[ ] first todo leaf{RESET}
1.3   [ ] second todo leaf
"
    );
    assert_eq!(stdout, expected);
}
