use super::*;
use tempfile::tempdir;

#[test]
fn live_config_overrides_disk_and_closing_restores_disk_content() {
    let dir = tempdir().unwrap();
    let task = dir.path().join("tasks.agile.md");
    let config = dir.path().join("mdagile.toml");
    let mut file_content = "\
[Properties.saved]
";
    std::fs::write(&config, file_content).unwrap();
    let disk_content = file_content;
    let mut docs = HashMap::new();
    file_content = "\
[Properties.unsaved]
";
    let config_uri = Url::from_file_path(&config).unwrap();
    docs.insert(config_uri.clone(), file_content.to_string());

    let source = resolve(Some(dir.path()), &task, &docs).unwrap().unwrap();
    assert_eq!(source.path, config);
    assert_eq!(source.text, file_content);

    docs.remove(&config_uri);
    let source = resolve(Some(dir.path()), &task, &docs).unwrap().unwrap();
    assert_eq!(source.text, disk_content);
}

#[test]
fn snapshot_detects_creation_removal_and_filename_switch() {
    let dir = tempdir().unwrap();
    let task = dir.path().join("tasks.agile.md");
    let docs = HashMap::new();
    let before = snapshot(&resolve(Some(dir.path()), &task, &docs));
    assert_eq!(before, Snapshot::Absent);

    let first = dir.path().join("mdagile.toml");
    let file_content = "\
[Properties.feature]
";
    std::fs::write(&first, file_content).unwrap();
    let created = snapshot(&resolve(Some(dir.path()), &task, &docs));
    assert_ne!(before, created);

    std::fs::remove_file(&first).unwrap();
    let second = dir.path().join(".mdagile.toml");
    std::fs::write(&second, file_content).unwrap();
    let switched = snapshot(&resolve(Some(dir.path()), &task, &docs));
    assert_ne!(created, switched);

    std::fs::remove_file(&second).unwrap();
    assert_eq!(snapshot(&resolve(Some(dir.path()), &task, &docs)), before);
}

#[test]
fn no_root_discovers_nearest_config_and_unsaved_conflicts() {
    let dir = tempdir().unwrap();
    let task = dir.path().join("nested").join("tasks.agile.md");
    std::fs::create_dir(task.parent().unwrap()).unwrap();
    let config = dir.path().join("mdagile.toml");
    let file_content = "\
[Properties.feature]
";
    std::fs::write(&config, file_content).unwrap();
    let mut docs = HashMap::new();
    let source = resolve(None, &task, &docs).unwrap().unwrap();
    assert_eq!(source.path, config);
    docs.insert(
        Url::from_file_path(dir.path().join(".mdagile.toml")).unwrap(),
        String::new(),
    );
    assert!(matches!(
        resolve(None, &task, &docs),
        Err(ConfigError::ConflictingConfig { .. })
    ));
}

#[test]
fn snapshot_tracks_disk_content_without_relying_on_mtime() {
    let dir = tempdir().unwrap();
    let task = dir.path().join("tasks.agile.md");
    let config = dir.path().join("mdagile.toml");
    let file_content = "\
[Properties.feature]
";
    std::fs::write(&config, file_content).unwrap();
    let docs = HashMap::new();
    let before = snapshot(&resolve(Some(dir.path()), &task, &docs));
    let file_content = "\
[Properties.other]
";
    std::fs::write(&config, file_content).unwrap();
    assert_ne!(snapshot(&resolve(Some(dir.path()), &task, &docs)), before);
}
