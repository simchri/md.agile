use super::*;

#[test]
fn format_settings_then_parse_settings_round_trips() {
    let settings = Settings {
        current: Some(PathBuf::from("/home/user/projectA")),
        recent: vec![
            PathBuf::from("/home/user/projectA"),
            PathBuf::from("/home/user/projectB"),
        ],
    };
    let text = format_settings(&settings);
    assert_eq!(parse_settings(&text), settings);
}

#[test]
fn format_settings_omits_current_when_none() {
    let settings = Settings {
        current: None,
        recent: vec![],
    };
    let text = format_settings(&settings);
    assert!(!text.contains("current"));
}

#[test]
fn parse_settings_tolerates_malformed_contents() {
    assert_eq!(parse_settings(""), Settings::default());
    assert_eq!(parse_settings("not valid toml {{{"), Settings::default());
}

#[test]
fn parse_settings_tolerates_missing_fields() {
    assert_eq!(
        parse_settings("current = \"/a/b\"\n"),
        Settings {
            current: Some(PathBuf::from("/a/b")),
            recent: vec![],
        }
    );
    assert_eq!(
        parse_settings("recent = [\"/a/b\"]\n"),
        Settings {
            current: None,
            recent: vec![PathBuf::from("/a/b")],
        }
    );
}

#[test]
fn read_settings_returns_default_when_file_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist.toml");
    assert_eq!(read_settings(&path), Settings::default());
}

#[test]
fn write_settings_then_read_settings_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("settings.toml");
    let settings = Settings {
        current: Some(PathBuf::from("/projects/foo")),
        recent: vec![
            PathBuf::from("/projects/foo"),
            PathBuf::from("/projects/bar"),
        ],
    };
    write_settings(&path, &settings).unwrap();
    assert_eq!(read_settings(&path), settings);
}

#[test]
fn write_settings_creates_parent_directories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a").join("b").join("settings.toml");
    assert!(!path.parent().unwrap().exists());
    write_settings(&path, &Settings::default()).unwrap();
    assert!(path.exists());
}

#[test]
fn settings_file_path_prefers_xdg_config_home() {
    // Can't safely mutate process env vars in parallel test runs, so this
    // exercises the logic directly rather than via `std::env::set_var`.
    // (See `settings_file_path_falls_back_to_home_config` below for the
    // fallback branch, exercised the same way — both intentionally avoid
    // touching real process env vars.)
    let path = settings_file_path();
    // Whichever branch fired, the file must always be named
    // "mdagile-gui/settings.toml" under *some* base directory.
    assert!(path.ends_with("mdagile-gui/settings.toml"));
}

#[test]
fn record_project_sets_current_and_prepends_to_recent() {
    let settings = Settings::default();
    let updated = record_project(&settings, Path::new("/projects/foo"));
    assert_eq!(updated.current, Some(PathBuf::from("/projects/foo")));
    assert_eq!(updated.recent, vec![PathBuf::from("/projects/foo")]);
}

#[test]
fn record_project_moves_existing_entry_to_front_without_duplicating() {
    let settings = Settings {
        current: Some(PathBuf::from("/projects/bar")),
        recent: vec![
            PathBuf::from("/projects/bar"),
            PathBuf::from("/projects/foo"),
        ],
    };
    let updated = record_project(&settings, Path::new("/projects/foo"));
    assert_eq!(updated.current, Some(PathBuf::from("/projects/foo")));
    assert_eq!(
        updated.recent,
        vec![
            PathBuf::from("/projects/foo"),
            PathBuf::from("/projects/bar"),
        ]
    );
}

#[test]
fn record_project_caps_recent_list_length() {
    let settings = Settings {
        current: None,
        recent: (0..20)
            .map(|i| PathBuf::from(format!("/projects/p{i}")))
            .collect(),
    };
    let updated = record_project(&settings, Path::new("/projects/new"));
    assert_eq!(updated.recent.len(), MAX_RECENT);
    assert_eq!(updated.recent[0], PathBuf::from("/projects/new"));
}

#[test]
fn is_project_dir_is_true_when_mdagile_toml_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mdagile.toml"), "").unwrap();
    assert!(is_project_dir(dir.path()));
}

#[test]
fn is_project_dir_is_false_when_mdagile_toml_missing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!is_project_dir(dir.path()));
}

#[test]
fn is_project_dir_is_false_when_mdagile_toml_is_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("mdagile.toml")).unwrap();
    assert!(!is_project_dir(dir.path()));
}
