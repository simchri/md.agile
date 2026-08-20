use super::*;
use crate::config::{Config, UserConfig};
use crate::formatter::{BOLD, RESET};
use crate::parser::{self, FileItem};
use crate::rules::ResolvedIdentity;

fn parse_one_task(file_content: &str) -> parser::Task {
    let items = parser::parse(file_content, PathBuf::from("t.agile.md"));
    for item in items {
        if let FileItem::Task(task) = item {
            return task;
        }
    }
    panic!("no top-level task found in {file_content:?}");
}

fn config_with_users(users: &[&str]) -> Config {
    Config {
        users: users
            .iter()
            .map(|&n| {
                (
                    n.to_string(),
                    UserConfig {
                        name: n.to_string(),
                        git_emails: vec![],
                        git_names: vec![],
                    },
                )
            })
            .collect(),
        ..Config::default()
    }
}

#[test]
fn render_task_highlighting_next_leaf_bolds_the_only_todo_leaf() {
    let file_content = "\
- [ ] parent task
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, None, &mut out);
    assert_eq!(out, format!("{BOLD}[ ] parent task{RESET}\n"));
}

#[test]
fn render_task_highlighting_next_leaf_bolds_first_todo_leaf_in_document_order() {
    let file_content = "\
- [ ] parent task
  - [x] already done subtask
  - [ ] first todo leaf
  - [ ] second todo leaf
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, None, &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] parent task\n  [x] already done subtask\n  {BOLD}[ ] first todo leaf{RESET}\n  [ ] second todo leaf\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_skips_non_leaf_todo_nodes() {
    // A `Todo` node with children is not itself a leaf, so it is never
    // bolded - only the actual leaf (a node with no children) is.
    let file_content = "\
- [ ] parent task
  - [ ] mid-level subtask with children
    - [ ] actual leaf
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, None, &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] parent task\n  [ ] mid-level subtask with children\n    {BOLD}[ ] actual leaf{RESET}\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_bolds_nothing_when_no_todo_leaf_exists() {
    let file_content = "\
- [x] parent task
  - [x] done subtask
  - [-] cancelled subtask
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, None, &mut out);
    assert_eq!(
        out,
        "[x] parent task\n  [x] done subtask\n  [-] cancelled subtask\n"
    );
    assert!(!out.contains(BOLD));
}

#[test]
fn render_subtask_as_root_highlighting_next_leaf_bolds_within_its_own_subtree() {
    let file_content = "\
- [ ] parent task
  - [ ] addressed subtask
    - [x] already done grandchild
    - [ ] next leaf
";
    let task = parse_one_task(file_content);
    let addressed = &task.children[0];
    let mut out = String::new();
    render_subtask_as_root_highlighting_next_leaf(addressed, false, None, &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] addressed subtask\n  [x] already done grandchild\n  {BOLD}[ ] next leaf{RESET}\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_includes_body_when_requested() {
    let file_content = "\
- [ ] parent task
  some body text
  - [ ] leaf with a body
    leaf body line
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, true, None, &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] parent task\n  some body text\n  {BOLD}[ ] leaf with a body{RESET}\n    leaf body line\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_omits_body_by_default() {
    let file_content = "\
- [ ] parent task
  some body text
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, None, &mut out);
    assert_eq!(out, format!("{BOLD}[ ] parent task{RESET}\n"));
}

#[test]
fn render_task_highlighting_next_leaf_skips_leaf_assigned_to_someone_else_when_identity_given() {
    // Regression test for the reported bug: bolding must respect eligibility
    // for the given identity, not just "first Todo leaf" unconditionally.
    let file_content = "\
- [ ] parent task
  - [ ] leaf assigned to bob @bob
  - [ ] leaf eligible for alice
";
    let task = parse_one_task(file_content);
    let config = config_with_users(&["alice", "bob"]);
    let identity = ResolvedIdentity::Known("alice".to_string());
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, Some((&identity, &config)), &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] parent task\n  [ ] leaf assigned to bob @bob\n  {BOLD}[ ] leaf eligible for alice{RESET}\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_bolds_nothing_when_no_leaf_eligible_for_identity() {
    let file_content = "\
- [ ] parent task
  - [ ] leaf assigned to bob @bob
";
    let task = parse_one_task(file_content);
    let config = config_with_users(&["alice", "bob"]);
    let identity = ResolvedIdentity::Known("alice".to_string());
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, Some((&identity, &config)), &mut out);
    assert_eq!(out, "[ ] parent task\n  [ ] leaf assigned to bob @bob\n");
    assert!(!out.contains(BOLD));
}

fn config_with_systemtest_property_and_users(users: &[&str]) -> Config {
    let mut config = config_with_users(users);
    config.properties.insert(
        "systemtest".to_string(),
        crate::config::PropertyConfig {
            name: "systemtest".to_string(),
            subtasks: vec![
                "1. write draft".to_string(),
                "2. assign review".to_string(),
                "3. implement feedback".to_string(),
                "4. approved".to_string(),
            ],
            subtasks_allow_cancel: vec![true, true, true, true],
        },
    );
    config
}

const VER_161_FILE_CONTENT: &str = "\
- [ ] VER-161 System Test Image Installation and Smoke Test #systemtest
  - [x] \"1. write draft\"
  - [ ] \"2. assign review\" @Gini
  - [ ] \"3. implement feedback\"
  - [ ] \"4. approved\"
";

#[test]
fn render_task_highlighting_next_leaf_bolds_first_todo_leaf_among_quoted_property_subtasks() {
    let task = parse_one_task(VER_161_FILE_CONTENT);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, None, &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] VER-161 System Test Image Installation and Smoke Test #systemtest\n  [x] 1. write draft\n  {BOLD}[ ] 2. assign review{RESET}\n  [ ] 3. implement feedback\n  [ ] 4. approved\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_bolds_assigned_leaf_for_its_own_assignee() {
    // With the `#systemtest` property config in play and the identity set to
    // the leaf's own assignee (Gini), the leaf assigned to her is eligible
    // and gets bolded, same as the unconditional case.
    let task = parse_one_task(VER_161_FILE_CONTENT);
    let config = config_with_systemtest_property_and_users(&["Gini"]);
    let identity = ResolvedIdentity::Known("Gini".to_string());
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, Some((&identity, &config)), &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] VER-161 System Test Image Installation and Smoke Test #systemtest\n  [x] 1. write draft\n  {BOLD}[ ] 2. assign review{RESET}\n  [ ] 3. implement feedback\n  [ ] 4. approved\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_skips_assigned_leaf_for_a_different_identity() {
    // For an identity other than Gini, the leaf assigned to her is skipped
    // over (but still printed unbolded); the next unassigned leaf ("3.
    // implement feedback", open to anyone) is bolded instead.
    let task = parse_one_task(VER_161_FILE_CONTENT);
    let config = config_with_systemtest_property_and_users(&["Gini", "alice"]);
    let identity = ResolvedIdentity::Known("alice".to_string());
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, Some((&identity, &config)), &mut out);
    assert_eq!(
        out,
        format!(
            "[ ] VER-161 System Test Image Installation and Smoke Test #systemtest\n  [x] 1. write draft\n  [ ] 2. assign review\n  {BOLD}[ ] 3. implement feedback{RESET}\n  [ ] 4. approved\n"
        )
    );
}

#[test]
fn render_task_highlighting_next_leaf_bolds_unconditionally_without_identity() {
    // Without an identity (plain `agile task next`, no `--mine`/`--as`), the
    // old unconditional "first Todo leaf" behavior still applies.
    let file_content = "\
- [ ] parent task
  - [ ] leaf assigned to bob @bob
";
    let task = parse_one_task(file_content);
    let mut out = String::new();
    render_task_highlighting_next_leaf(&task, false, None, &mut out);
    assert_eq!(
        out,
        format!("[ ] parent task\n  {BOLD}[ ] leaf assigned to bob @bob{RESET}\n")
    );
}
