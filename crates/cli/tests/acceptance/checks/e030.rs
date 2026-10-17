//! Acceptance tests for E030 "Form Validation Failed" and E031 "Form Script
//! Failed": `agile check` runs the `validation` script of a task's form, but
//! only once it was approved with `agile trust`.

use crate::helpers::run_agile_clean_env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::tempdir;

fn write_project(dir: &Path, script_content: &str) {
    let config = "\
[Field.email]
label = \"Email\"

[Form.personal_info]
fields = [\"email\"]
validation = \"validate.sh\"

[Properties.onboarding]
form = \"personal_info\"
";
    fs::write(dir.join("mdagile.toml"), config).unwrap();
    fs::write(dir.join("validate.sh"), script_content).unwrap();
    fs::set_permissions(dir.join("validate.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    let file_content = "\
- [ ] new colleague #onboarding
  __Email:__ john
";
    fs::write(dir.join("tasks.agile.md"), file_content).unwrap();
}

fn check(project: &Path, config_home: &Path) -> std::process::Output {
    run_agile_clean_env(
        project,
        &["check"],
        &[("XDG_CONFIG_HOME", config_home.to_str().unwrap())],
    )
}

#[test]
fn a_trusted_script_reporting_a_field_error_fails_the_check() {
    // Arrange
    let project = tempdir().unwrap();
    let config_home = tempdir().unwrap();
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false, \"errors\": [{\"field\": \"email\", \"message\": \"not an email\"}]}'
";
    write_project(project.path(), script_content);
    let trust = run_agile_clean_env(
        project.path(),
        &["trust", "--yes"],
        &[("XDG_CONFIG_HOME", config_home.path().to_str().unwrap())],
    );
    assert_eq!(trust.status.code(), Some(0), "{trust:?}");

    // Act
    let out = check(project.path(), config_home.path());

    // Assert
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E030"), "stdout: {stdout:?}");
    assert!(stdout.contains("not an email"), "stdout: {stdout:?}");
    assert!(stdout.contains(":2:"), "stdout: {stdout:?}");
}

#[test]
fn a_trusted_script_reporting_valid_passes_the_check() {
    // Arrange
    let project = tempdir().unwrap();
    let config_home = tempdir().unwrap();
    let script_content = "\
#!/bin/sh
echo '{\"valid\": true}'
";
    write_project(project.path(), script_content);
    run_agile_clean_env(
        project.path(),
        &["trust", "--yes"],
        &[("XDG_CONFIG_HOME", config_home.path().to_str().unwrap())],
    );

    // Act
    let out = check(project.path(), config_home.path());

    // Assert
    assert_eq!(out.status.code(), Some(0), "{out:?}");
}

#[test]
fn an_untrusted_script_fails_the_check_with_trust_instructions() {
    // Arrange
    let project = tempdir().unwrap();
    let config_home = tempdir().unwrap();
    let script_content = "\
#!/bin/sh
echo '{\"valid\": true}'
";
    write_project(project.path(), script_content);

    // Act
    let out = check(project.path(), config_home.path());

    // Assert
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E031"), "stdout: {stdout:?}");
    assert!(stdout.contains("agile trust --yes"), "stdout: {stdout:?}");
}

#[test]
fn a_script_edited_after_trusting_is_untrusted_again() {
    // Arrange
    let project = tempdir().unwrap();
    let config_home = tempdir().unwrap();
    let script_content = "\
#!/bin/sh
echo '{\"valid\": true}'
";
    write_project(project.path(), script_content);
    run_agile_clean_env(
        project.path(),
        &["trust", "--yes"],
        &[("XDG_CONFIG_HOME", config_home.path().to_str().unwrap())],
    );
    let edited_content = "\
#!/bin/sh
echo '{\"valid\": true}' # edited
";
    fs::write(project.path().join("validate.sh"), edited_content).unwrap();

    // Act
    let out = check(project.path(), config_home.path());

    // Assert
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E031"), "stdout: {stdout:?}");
}
