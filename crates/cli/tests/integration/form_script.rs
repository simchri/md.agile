use mdagile::form_script::{Limits, RunError, ScriptInput, TaskInfo, run_script};
use mdagile::trust::{TrustStore, sha256_hex};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

fn input() -> ScriptInput {
    ScriptInput {
        form: "personal_info".to_string(),
        task: TaskInfo {
            title: "new colleague".to_string(),
            line: 1,
        },
        fields: [("Email".to_string(), "not-an-email".to_string())].into(),
    }
}

#[test]
fn a_trusted_script_validates_a_form_end_to_end() {
    // Arrange
    let dir = tempdir().unwrap();
    let script_content = "\
#!/bin/sh
cat > /dev/null
echo '{\"valid\": false, \"errors\": [{\"field\": \"Email\", \"message\": \"not an email address\"}]}'
";
    let path = dir.path().join("validate.sh");
    fs::write(&path, script_content).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let mut store = TrustStore::default();
    store.trust("validate.sh", &sha256_hex(script_content.as_bytes()));

    // Act
    let output = run_script(
        dir.path(),
        "validate.sh",
        &input(),
        &store,
        &Limits::default(),
    )
    .unwrap();

    // Assert
    assert!(!output.valid);
    assert_eq!(output.errors[0].field.as_deref(), Some("Email"));
    assert_eq!(output.errors[0].message, "not an email address");
}

#[test]
fn an_unapproved_script_is_refused() {
    // Arrange
    let dir = tempdir().unwrap();
    let script_content = "\
#!/bin/sh
echo '{\"valid\": true}'
";
    let path = dir.path().join("validate.sh");
    fs::write(&path, script_content).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();

    // Act
    let result = run_script(
        dir.path(),
        "validate.sh",
        &input(),
        &TrustStore::default(),
        &Limits::default(),
    );

    // Assert
    assert!(matches!(result, Err(RunError::Untrusted)));
}
