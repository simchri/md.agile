use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn sha256_hex_matches_known_digest() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn loading_a_missing_store_gives_an_empty_store() {
    let dir = tempdir().unwrap();

    let store = TrustStore::load(&dir.path().join("trust.json")).unwrap();

    assert!(!store.is_trusted("validate.sh", "abc"));
}

#[test]
fn trusted_entries_survive_a_save_and_load_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested").join("trust.json");
    let mut store = TrustStore::default();
    store.trust("scripts/validate.sh", "abc");

    store.save(&path).unwrap();
    let loaded = TrustStore::load(&path).unwrap();

    assert!(loaded.is_trusted("scripts/validate.sh", "abc"));
}

#[test]
fn a_changed_hash_or_path_is_not_trusted() {
    let mut store = TrustStore::default();
    store.trust("validate.sh", "abc");

    assert!(!store.is_trusted("validate.sh", "different"));
    assert!(!store.is_trusted("other.sh", "abc"));
}

#[test]
fn loading_a_malformed_store_is_an_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("trust.json");
    let file_content = "not json";
    fs::write(&path, file_content).unwrap();

    assert!(TrustStore::load(&path).is_err());
}

#[test]
fn store_path_prefers_xdg_config_home_over_home() {
    let env = |name: &str| match name {
        "XDG_CONFIG_HOME" => Some("/xdg".into()),
        "HOME" => Some("/home/u".into()),
        _ => None,
    };

    assert_eq!(
        default_store_path(env),
        Some(PathBuf::from("/xdg/mdagile/trust.json"))
    );
}

#[test]
fn store_path_falls_back_to_dot_config_under_home() {
    let env = |name: &str| match name {
        "HOME" => Some("/home/u".into()),
        _ => None,
    };

    assert_eq!(
        default_store_path(env),
        Some(PathBuf::from("/home/u/.config/mdagile/trust.json"))
    );
}

#[test]
fn store_path_is_none_without_xdg_config_home_or_home() {
    assert_eq!(default_store_path(|_| None), None);
}

#[test]
fn resolve_script_accepts_a_file_inside_the_project() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("scripts")).unwrap();
    let file_content = "#!/bin/sh\n";
    fs::write(dir.path().join("scripts/validate.sh"), file_content).unwrap();

    let resolved = resolve_script(dir.path(), "scripts/validate.sh").unwrap();

    assert!(resolved.ends_with("scripts/validate.sh"));
}

#[test]
fn resolve_script_rejects_absolute_paths() {
    let dir = tempdir().unwrap();

    assert_eq!(
        resolve_script(dir.path(), "/bin/sh"),
        Err(ScriptPathError::Absolute)
    );
}

#[test]
fn resolve_script_rejects_parent_traversal() {
    let dir = tempdir().unwrap();

    assert_eq!(
        resolve_script(dir.path(), "../evil.sh"),
        Err(ScriptPathError::OutsideProject)
    );
}

#[test]
fn resolve_script_rejects_a_missing_file() {
    let dir = tempdir().unwrap();

    assert_eq!(
        resolve_script(dir.path(), "missing.sh"),
        Err(ScriptPathError::NotFound)
    );
}

#[cfg(unix)]
#[test]
fn resolve_script_rejects_a_symlink_escaping_the_project() {
    let project = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let file_content = "#!/bin/sh\n";
    fs::write(outside.path().join("evil.sh"), file_content).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("evil.sh"),
        project.path().join("link.sh"),
    )
    .unwrap();

    assert_eq!(
        resolve_script(project.path(), "link.sh"),
        Err(ScriptPathError::OutsideProject)
    );
}
