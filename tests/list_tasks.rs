use mdagile::list_tasks;

// ── basic task listing ────────────────────────────────────────────────────────

const BASIC_INPUT: &str = "\
- [ ] implement feature X
  - [x] subtask one
  - [ ] subtask two

- [x] another task
- [-] a cancelled task
";

const BASIC_EXPECTED: &str = "\
[ ] implement feature X
  [x] subtask one
  [ ] subtask two
[x] another task
[-] a cancelled task
";

#[test]
fn list_tasks_basic() {
    assert_eq!(list_tasks(BASIC_INPUT), BASIC_EXPECTED);
}

// ── other content (headings, paragraphs) is ignored ──────────────────────────

const OTHER_CONTENT_INPUT: &str = "\
# Sprint backlog

Some notes about the project.

- [ ] first task
- [x] second task

## Done

Nice work everyone.

- [-] third task (cancelled)
";

const OTHER_CONTENT_EXPECTED: &str = "\
[ ] first task
[x] second task
[-] third task (cancelled)
";

#[test]
fn other_content_is_ignored() {
    assert_eq!(list_tasks(OTHER_CONTENT_INPUT), OTHER_CONTENT_EXPECTED);
}

// ── task body text is not listed ─────────────────────────────────────────────

const BODY_TEXT_INPUT: &str = "\
- [ ] implement feature X
  This is the task body. Some details about the task.
  More detail on another line.
  - [ ] a subtask

- [x] another task
  Body text here too.
";

const BODY_TEXT_EXPECTED: &str = "\
[ ] implement feature X
  [ ] a subtask
[x] another task
";

#[test]
fn task_body_text_not_listed() {
    assert_eq!(list_tasks(BODY_TEXT_INPUT), BODY_TEXT_EXPECTED);
}
