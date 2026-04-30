use mdagile::parser::{self, FileItem};
use mdagile::{find_task_files, format_file_list, list_task_blocks};
use std::path::PathBuf;

fn p(input: &str) -> Vec<FileItem> {
    parser::parse(input, PathBuf::from("test.agile.md"))
}

// ── --last on task list ───────────────────────────────────────────────────────

#[test]
fn last_n_tasks_returns_trailing_blocks() {
    let input = "\
- [ ] task one
  - [ ] subtask
- [x] task two
- [ ] task three
";
    let expected = "\
[x] task two
[ ] task three
";
    let blocks = list_task_blocks(&p(input));
    let skip = blocks.len().saturating_sub(2);
    let result: String = blocks.into_iter().skip(skip).collect();
    assert_eq!(result, expected);
}

#[test]
fn last_n_larger_than_total_returns_all() {
    let input = "\
- [ ] task one
- [ ] task two
";
    let expected = "\
[ ] task one
[ ] task two
";
    let blocks = list_task_blocks(&p(input));
    let skip = blocks.len().saturating_sub(99);
    let result: String = blocks.into_iter().skip(skip).collect();
    assert_eq!(result, expected);
}

// ── --last on file list ───────────────────────────────────────────────────────

#[test]
fn last_n_files_returns_trailing_entries() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.agile.md"), "").unwrap();
    fs::write(dir.path().join("b.agile.md"), "").unwrap();
    fs::write(dir.path().join("c.agile.md"), "").unwrap();

    let paths = find_task_files(dir.path());
    let skip = paths.len().saturating_sub(2);
    let result = format_file_list(&paths[skip..]);
    let expected = format!(
        "b.agile.md  {}\nc.agile.md  {}\n",
        dir.path().join("b.agile.md").display(),
        dir.path().join("c.agile.md").display(),
    );
    assert_eq!(result, expected);
}
use std::fs;
use tempfile::tempdir;

// ── list_task_blocks ──────────────────────────────────────────────────────────

#[test]
fn splits_into_one_block_per_top_level_task() {
    let input = "\
- [ ] task one
  - [ ] subtask
- [x] task two
- [ ] task three
";
    let blocks = list_task_blocks(&p(input));
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0], "[ ] task one\n  [ ] subtask\n");
    assert_eq!(blocks[1], "[x] task two\n");
    assert_eq!(blocks[2], "[ ] task three\n");
}

#[test]
fn empty_input_gives_no_blocks() {
    assert_eq!(list_task_blocks(&p("")), Vec::<String>::new());
}

// ── --next on task list ───────────────────────────────────────────────────────

#[test]
fn first_n_tasks_returns_leading_blocks() {
    let input = "\
- [ ] task one
  - [ ] subtask
- [x] task two
- [ ] task three
";
    let expected = "\
[ ] task one
  [ ] subtask
[x] task two
";
    let result: String = list_task_blocks(&p(input)).into_iter().take(2).collect();
    assert_eq!(result, expected);
}

#[test]
fn first_n_larger_than_total_returns_all() {
    let input = "\
- [ ] task one
- [ ] task two
";
    let expected = "\
[ ] task one
[ ] task two
";
    let result: String = list_task_blocks(&p(input)).into_iter().take(99).collect();
    assert_eq!(result, expected);
}

// ── --next on file list ───────────────────────────────────────────────────────

#[test]
fn first_n_files_returns_leading_entries() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.agile.md"), "").unwrap();
    fs::write(dir.path().join("b.agile.md"), "").unwrap();
    fs::write(dir.path().join("c.agile.md"), "").unwrap();

    let paths = find_task_files(dir.path());
    let result = format_file_list(&paths[..2]);
    let expected = format!(
        "a.agile.md  {}\nb.agile.md  {}\n",
        dir.path().join("a.agile.md").display(),
        dir.path().join("b.agile.md").display(),
    );
    assert_eq!(result, expected);
}
