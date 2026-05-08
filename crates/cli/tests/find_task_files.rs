use mdagile::cli::common::find_task_files;
use mdagile::cli::subcommands::list::format_file_list;
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
    assert_eq!(filenames(&files), vec!["bravo.agile.md", "charlie.agile.md", "alpha.agile.md"]);
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
    assert_eq!(find_task_files(dir.path()), Vec::<std::path::PathBuf>::new());
}
