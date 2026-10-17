use crate::helpers::run_agile_clean_env;
use std::fs;
use tempfile::tempdir;

fn write_project(dir: &std::path::Path) {
    let config = "\
[Field.email]

[Form.personal_info]
fields = [\"email\"]
validation = \"validate.sh\"
";
    fs::write(dir.join("mdagile.toml"), config).unwrap();
    let script_content = "\
#!/bin/sh
echo '{\"valid\": true}'
";
    fs::write(dir.join("validate.sh"), script_content).unwrap();
}

#[test]
fn trust_yes_trusts_scripts_and_writes_the_per_user_store() {
    // Arrange
    let project = tempdir().unwrap();
    let config_home = tempdir().unwrap();
    write_project(project.path());

    // Act
    let out = run_agile_clean_env(
        project.path(),
        &["trust", "--yes"],
        &[("XDG_CONFIG_HOME", config_home.path().to_str().unwrap())],
    );

    // Assert
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "trusted: validate.sh\n"
    );
    let store = fs::read_to_string(config_home.path().join("mdagile/trust.json")).unwrap();
    assert!(store.contains("\"script\": \"validate.sh\""), "{store}");
    assert!(store.contains("\"sha256\""), "{store}");
}

#[test]
fn trust_without_yes_and_without_a_terminal_fails_with_instructions() {
    // Arrange
    let project = tempdir().unwrap();
    let config_home = tempdir().unwrap();
    write_project(project.path());

    // Act
    let out = run_agile_clean_env(
        project.path(),
        &["trust"],
        &[("XDG_CONFIG_HOME", config_home.path().to_str().unwrap())],
    );

    // Assert
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("agile trust --yes"), "{stderr}");
    assert!(!config_home.path().join("mdagile/trust.json").exists());
}

#[test]
fn trust_reports_an_already_trusted_script_on_the_second_run() {
    // Arrange
    let project = tempdir().unwrap();
    let config_home = tempdir().unwrap();
    write_project(project.path());
    let env = [("XDG_CONFIG_HOME", config_home.path().to_str().unwrap())];
    run_agile_clean_env(project.path(), &["trust", "--yes"], &env);

    // Act
    let out = run_agile_clean_env(project.path(), &["trust", "--yes"], &env);

    // Assert
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "already trusted: validate.sh\n"
    );
}

#[test]
fn editing_a_script_revokes_trust() {
    // Arrange
    let project = tempdir().unwrap();
    let config_home = tempdir().unwrap();
    write_project(project.path());
    let env = [("XDG_CONFIG_HOME", config_home.path().to_str().unwrap())];
    run_agile_clean_env(project.path(), &["trust", "--yes"], &env);
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false}'
";
    fs::write(project.path().join("validate.sh"), script_content).unwrap();

    // Act
    let out = run_agile_clean_env(project.path(), &["trust", "--yes"], &env);

    // Assert
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "trusted: validate.sh\n"
    );
}
