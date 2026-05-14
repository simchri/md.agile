use mdagile::cli::common::{find_task_files, parse_files};
use mdagile::cli::subcommands::list::format_file_list;
use mdagile::parser::FileItem;
use std::fs;
use tempfile::tempdir;

fn filenames(paths: &[std::path::PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect()
}

#[test]
fn returns_only_agile_md_files() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("tasks.agile.md"), "").unwrap();
    fs::write(dir.path().join("README.md"), "").unwrap();
    fs::write(dir.path().join("notes.txt"), "").unwrap();

    let files = find_task_files(dir.path());
    assert_eq!(filenames(&files), vec!["tasks.agile.md"]);
}

#[test]
fn directory_prefix_determines_order() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("zzz-subdir");
    fs::create_dir(&sub).unwrap();

    fs::write(dir.path().join("charlie.agile.md"), "").unwrap();
    fs::write(sub.join("alpha.agile.md"), "").unwrap(); // 'z' subdir sorts after root files
    fs::write(dir.path().join("bravo.agile.md"), "").unwrap();

    let files = find_task_files(dir.path());
    // root-level files (bravo, charlie) sort before zzz-subdir/alpha
    assert_eq!(
        filenames(&files),
        vec!["bravo.agile.md", "charlie.agile.md", "alpha.agile.md"]
    );
}

#[test]
fn finds_files_in_subdirectories() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("subdir");
    fs::create_dir(&sub).unwrap();

    fs::write(dir.path().join("root.agile.md"), "").unwrap();
    fs::write(sub.join("nested.agile.md"), "").unwrap();

    let files = find_task_files(dir.path());
    // root.agile.md sorts before subdir/nested.agile.md ('r' < 's')
    assert_eq!(filenames(&files), vec!["root.agile.md", "nested.agile.md"]);
}

#[test]
fn format_file_list_shows_filename_and_full_path() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    fs::write(dir.path().join("beta.agile.md"), "").unwrap();
    fs::write(sub.join("alpha.agile.md"), "").unwrap();

    let paths = find_task_files(dir.path());
    // beta.agile.md is at root ('b' < 's'), so it comes before subdir/alpha.agile.md
    let expected = format!(
        "beta.agile.md  {}\nalpha.agile.md  {}\n",
        dir.path().join("beta.agile.md").display(),
        sub.join("alpha.agile.md").display(),
    );
    assert_eq!(format_file_list(&paths), expected);
}

#[test]
fn format_file_list_empty() {
    assert_eq!(format_file_list(&[]), "".to_string());
}

#[test]
fn empty_directory_returns_no_files() {
    let dir = tempdir().unwrap();
    assert_eq!(
        find_task_files(dir.path()),
        Vec::<std::path::PathBuf>::new()
    );
}

#[test]
fn finds_tasks_via_symlinked_files() {
    let real_dir = tempdir().unwrap();
    let link_dir = tempdir().unwrap();

    fs::write(
        real_dir.path().join("tasks.agile.md"),
        "\
- [ ] my task from symlinked file
",
    )
    .unwrap();

    std::os::unix::fs::symlink(
        real_dir.path().join("tasks.agile.md"),
        link_dir.path().join("tasks.agile.md"),
    )
    .unwrap();

    let files = find_task_files(link_dir.path());
    assert_eq!(filenames(&files), vec!["tasks.agile.md"]);

    let items = parse_files(&files);
    let task = items.iter().find_map(|item| match item {
        FileItem::Task(t) => Some(t),
        _ => None,
    });
    assert!(task.is_some(), "no task parsed from symlinked file");
    assert_eq!(task.unwrap().title, "my task from symlinked file");
}

#[test]
fn finds_tasks_inside_symlinked_directory() {
    let real_dir = tempdir().unwrap();
    let root_dir = tempdir().unwrap();

    fs::write(
        real_dir.path().join("tasks.agile.md"),
        "\
- [ ] my task inside symlinked directory
",
    )
    .unwrap();

    // root_dir/linked -> real_dir  (directory symlink)
    std::os::unix::fs::symlink(real_dir.path(), root_dir.path().join("linked")).unwrap();

    let files = find_task_files(root_dir.path());
    assert_eq!(filenames(&files), vec!["tasks.agile.md"]);

    let items = parse_files(&files);
    let task = items.iter().find_map(|item| match item {
        FileItem::Task(t) => Some(t),
        _ => None,
    });
    assert!(task.is_some(), "no task parsed from file inside symlinked directory");
    assert_eq!(task.unwrap().title, "my task inside symlinked directory");
}

#[test]
fn finds_tasks_even_when_gitignored_by_parent() {
    // Reproduces the real-world case: MDAGILE_WORKDIR points at a directory
    // that a parent repo's .gitignore excludes.  The walker must not honour
    // that rule — task files should always be found.
    let parent_dir = tempdir().unwrap();
    let work_dir = parent_dir.path().join("myproject");
    fs::create_dir(&work_dir).unwrap();

    // Make parent_dir look like a git repo so the ignore crate reads its .gitignore
    fs::create_dir(parent_dir.path().join(".git")).unwrap();
    // Parent repo ignores "myproject/"
    fs::write(parent_dir.path().join(".gitignore"), "/myproject\n").unwrap();

    fs::write(
        work_dir.join("tasks.agile.md"),
        "\
- [ ] task that must not be gitignored away
",
    )
    .unwrap();

    let files = find_task_files(&work_dir);
    assert_eq!(
        filenames(&files),
        vec!["tasks.agile.md"],
        "file was hidden by parent .gitignore"
    );
}
