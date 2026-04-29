use mdagile::{find_task_files, format_file_list};
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
fn sorted_by_filename_ignoring_path() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("zzz-subdir");
    fs::create_dir(&sub).unwrap();

    fs::write(dir.path().join("charlie.agile.md"), "").unwrap();
    fs::write(sub.join("alpha.agile.md"), "").unwrap();    // deep path but 'a' sorts first
    fs::write(dir.path().join("bravo.agile.md"), "").unwrap();

    let files = find_task_files(dir.path());
    assert_eq!(filenames(&files), vec!["alpha.agile.md", "bravo.agile.md", "charlie.agile.md"]);
}

#[test]
fn finds_files_in_subdirectories() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("subdir");
    fs::create_dir(&sub).unwrap();

    fs::write(dir.path().join("root.agile.md"), "").unwrap();
    fs::write(sub.join("nested.agile.md"), "").unwrap();

    let files = find_task_files(dir.path());
    assert_eq!(filenames(&files), vec!["nested.agile.md", "root.agile.md"]);
}

#[test]
fn format_file_list_shows_filename_and_full_path() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    fs::write(dir.path().join("beta.agile.md"), "").unwrap();
    fs::write(sub.join("alpha.agile.md"), "").unwrap();

    let paths = find_task_files(dir.path());
    let expected = format!(
        "alpha.agile.md  {}\nbeta.agile.md  {}\n",
        sub.join("alpha.agile.md").display(),
        dir.path().join("beta.agile.md").display(),
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
