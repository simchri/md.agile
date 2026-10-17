use super::*;
use crate::config::Config;
use crate::trust::sha256_hex;
use std::fs;
use std::io::Cursor;
use tempfile::tempdir;

const CONFIG: &str = "\
[Field.email]
[Form.personal_info]
fields = [\"email\"]
validation = \"validate.sh\"
";

fn setup() -> (tempfile::TempDir, Config) {
    let dir = tempdir().unwrap();
    let file_content = "#!/bin/sh\n";
    fs::write(dir.path().join("validate.sh"), file_content).unwrap();
    (dir, Config::from_str(CONFIG).unwrap())
}

fn is_trusted(store_path: &Path, dir: &Path) -> bool {
    let hash = sha256_hex(&fs::read(dir.join("validate.sh")).unwrap());
    TrustStore::load(store_path)
        .unwrap()
        .is_trusted("validate.sh", &hash)
}

#[test]
fn yes_trusts_every_declared_script_without_prompting() {
    let (dir, config) = setup();
    let store_path = dir.path().join("store.json");
    let mut out = Vec::new();

    let result = run_with(
        dir.path(),
        &config,
        &Options {
            yes: true,
            interactive: false,
            store_path: &store_path,
        },
        &mut Cursor::new(""),
        &mut out,
    );

    assert_eq!(result, Ok(()));
    assert!(is_trusted(&store_path, dir.path()));
    assert_eq!(String::from_utf8(out).unwrap(), "trusted: validate.sh\n");
}

#[test]
fn non_interactive_without_yes_is_an_error_and_trusts_nothing() {
    let (dir, config) = setup();
    let store_path = dir.path().join("store.json");

    let result = run_with(
        dir.path(),
        &config,
        &Options {
            yes: false,
            interactive: false,
            store_path: &store_path,
        },
        &mut Cursor::new("y\n"),
        &mut Vec::new(),
    );

    assert!(result.unwrap_err().contains("agile trust --yes"));
    assert!(!store_path.exists());
}

#[test]
fn interactive_answer_y_trusts_the_script() {
    let (dir, config) = setup();
    let store_path = dir.path().join("store.json");
    let mut out = Vec::new();

    let result = run_with(
        dir.path(),
        &config,
        &Options {
            yes: false,
            interactive: true,
            store_path: &store_path,
        },
        &mut Cursor::new("y\n"),
        &mut out,
    );

    assert_eq!(result, Ok(()));
    assert!(is_trusted(&store_path, dir.path()));
}

#[test]
fn interactive_answer_other_than_y_skips_the_script() {
    let (dir, config) = setup();
    let store_path = dir.path().join("store.json");
    let mut out = Vec::new();

    let result = run_with(
        dir.path(),
        &config,
        &Options {
            yes: false,
            interactive: true,
            store_path: &store_path,
        },
        &mut Cursor::new("n\n"),
        &mut out,
    );

    assert_eq!(result, Ok(()));
    assert!(!store_path.exists());
    assert!(
        String::from_utf8(out)
            .unwrap()
            .contains("skipped: validate.sh")
    );
}

#[test]
fn an_already_trusted_script_is_reported_and_not_prompted_for() {
    let (dir, config) = setup();
    let store_path = dir.path().join("store.json");
    let options = Options {
        yes: true,
        interactive: false,
        store_path: &store_path,
    };
    run_with(
        dir.path(),
        &config,
        &options,
        &mut Cursor::new(""),
        &mut Vec::new(),
    )
    .unwrap();
    let mut out = Vec::new();

    let result = run_with(
        dir.path(),
        &config,
        &Options {
            yes: false,
            interactive: false,
            store_path: &store_path,
        },
        &mut Cursor::new(""),
        &mut out,
    );

    assert_eq!(result, Ok(()));
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "already trusted: validate.sh\n"
    );
}

#[test]
fn a_script_path_outside_the_project_is_an_error() {
    let dir = tempdir().unwrap();
    let file_content = "\
[Field.email]
[Form.personal_info]
fields = [\"email\"]
validation = \"../evil.sh\"
";
    let config = Config::from_str(file_content).unwrap();
    let store_path = dir.path().join("store.json");

    let result = run_with(
        dir.path(),
        &config,
        &Options {
            yes: true,
            interactive: false,
            store_path: &store_path,
        },
        &mut Cursor::new(""),
        &mut Vec::new(),
    );

    assert!(result.unwrap_err().contains("../evil.sh"));
}

#[test]
fn no_declared_scripts_prints_a_note() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("store.json");
    let mut out = Vec::new();

    let result = run_with(
        dir.path(),
        &Config::default(),
        &Options {
            yes: true,
            interactive: false,
            store_path: &store_path,
        },
        &mut Cursor::new(""),
        &mut out,
    );

    assert_eq!(result, Ok(()));
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "No form validation scripts are declared.\n"
    );
}
