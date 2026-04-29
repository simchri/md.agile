use mdagile::list_tasks;

const INPUT: &str = "\
- [ ] implement feature X
  - [x] subtask one
  - [ ] subtask two

- [x] another task
- [-] a cancelled task
";

const EXPECTED: &str = "\
[ ] implement feature X
  [x] subtask one
  [ ] subtask two
[x] another task
[-] a cancelled task
";

#[test]
fn list_tasks_basic() {
    assert_eq!(list_tasks(INPUT), EXPECTED);
}
