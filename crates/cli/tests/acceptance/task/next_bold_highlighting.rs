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

- [ ] foo
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
            "{BOLD}[ ] foo{RESET}\n"
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
    // For an identity other than Gini, the leaf assigned to her is skipped
    // over (but still printed unbolded); the next unassigned leaf ("3.
    // implement feedback", open to anyone) is bolded instead.
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    fs::write(dir.path().join("mdagile.toml"), VER_161_CONFIG).unwrap();
    fs::write(dir.path().join("tasks.agile.md"), VER_161_FILE_CONTENT).unwrap();

    let stdout = stdout_of(dir.path(), &["task", "next", "--as", "alice"]);
    assert_eq!(
        stdout,
        format!(
            "[ ] VER-161 System Test Image Installation and Smoke Test #systemtest\n  [x] 1. write draft\n  [ ] 2. assign review\n  {BOLD}[ ] 3. implement feedback{RESET}\n  [ ] 4. approved\n"
        )
    );
}
