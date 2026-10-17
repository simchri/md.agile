use super::*;
use crate::trust::{TrustStore, sha256_hex};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Instant;
use tempfile::{TempDir, tempdir};

fn input() -> ScriptInput {
    ScriptInput {
        form: "personal_info".to_string(),
        task: TaskInfo {
            title: "new colleague".to_string(),
            line: 3,
        },
        fields: [("Email".to_string(), "a@b".to_string())].into(),
    }
}

/// Writes an executable script and returns a project dir plus a store that trusts it.
fn project_with_script(script_content: &str) -> (TempDir, TrustStore) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("validate.sh");
    fs::write(&path, script_content).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let mut store = TrustStore::default();
    store.trust("validate.sh", &sha256_hex(script_content.as_bytes()));
    (dir, store)
}

fn run(script_content: &str) -> Result<ScriptOutput, RunError> {
    let (dir, store) = project_with_script(script_content);
    run_script(
        dir.path(),
        "validate.sh",
        &input(),
        &store,
        &Limits::default(),
    )
}

#[test]
fn a_script_reporting_valid_passes() {
    let script_content = "\
#!/bin/sh
echo '{\"valid\": true}'
";

    let output = run(script_content).unwrap();

    assert_eq!(
        output,
        ScriptOutput {
            valid: true,
            errors: vec![]
        }
    );
}

#[test]
fn a_script_reporting_invalid_returns_field_errors_with_exit_zero() {
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false, \"errors\": [{\"field\": \"Email\", \"message\": \"bad\"}, {\"message\": \"general\"}]}'
";

    let output = run(script_content).unwrap();

    assert_eq!(
        output,
        ScriptOutput {
            valid: false,
            errors: vec![
                ScriptError {
                    field: Some("Email".to_string()),
                    message: "bad".to_string()
                },
                ScriptError {
                    field: None,
                    message: "general".to_string()
                },
            ],
        }
    );
}

#[test]
fn the_script_receives_the_input_json_on_stdin() {
    let script_content = "\
#!/bin/sh
if grep -q '\"form\":\"personal_info\"'; then
  echo '{\"valid\": true}'
else
  echo '{\"valid\": false}'
fi
";

    assert!(run(script_content).unwrap().valid);
}

#[test]
fn the_script_receives_only_the_form_fields_as_label_value_pairs() {
    let script_content = "\
#!/bin/sh
input=$(cat)
case \"$input\" in
  *'\"fields\":{\"Email\":\"a@b\"}'*) echo '{\"valid\": true}' ;;
  *) echo '{\"valid\": false}' ;;
esac
";

    assert!(run(script_content).unwrap().valid);
}

#[test]
fn an_untrusted_script_is_never_executed() {
    let script_content = "\
#!/bin/sh
touch ran.marker
echo '{\"valid\": true}'
";
    let (dir, _) = project_with_script(script_content);

    let result = run_script(
        dir.path(),
        "validate.sh",
        &input(),
        &TrustStore::default(),
        &Limits::default(),
    );

    assert!(matches!(result, Err(RunError::Untrusted)));
    assert!(!dir.path().join("ran.marker").exists());
}

#[test]
fn a_script_edited_after_approval_is_untrusted() {
    let original = "\
#!/bin/sh
echo '{\"valid\": true}'
";
    let (dir, store) = project_with_script(original);
    let edited = "\
#!/bin/sh
echo '{\"valid\": false}'
";
    fs::write(dir.path().join("validate.sh"), edited).unwrap();

    let result = run_script(
        dir.path(),
        "validate.sh",
        &input(),
        &store,
        &Limits::default(),
    );

    assert!(matches!(result, Err(RunError::Untrusted)));
}

#[test]
fn a_script_path_outside_the_project_is_rejected() {
    let dir = tempdir().unwrap();

    let result = run_script(
        dir.path(),
        "../evil.sh",
        &input(),
        &TrustStore::default(),
        &Limits::default(),
    );

    assert!(matches!(result, Err(RunError::Path(_))));
}

#[test]
fn a_non_zero_exit_is_a_script_failure_with_stderr() {
    let script_content = "\
#!/bin/sh
echo boom >&2
exit 3
";

    match run(script_content) {
        Err(RunError::NonZeroExit { code, stderr }) => {
            assert_eq!(code, Some(3));
            assert_eq!(stderr.trim(), "boom");
        }
        other => panic!("expected NonZeroExit, got {other:?}"),
    }
}

#[test]
fn output_that_is_not_the_expected_json_is_a_script_failure() {
    let script_content = "\
#!/bin/sh
echo 'all good'
";

    assert!(matches!(
        run(script_content),
        Err(RunError::InvalidOutput(_))
    ));
}

#[test]
fn a_hanging_script_is_killed_after_the_timeout() {
    let script_content = "\
#!/bin/sh
sleep 30
";
    let (dir, store) = project_with_script(script_content);
    let limits = Limits {
        timeout: Duration::from_millis(200),
        ..Limits::default()
    };
    let started = Instant::now();

    let result = run_script(dir.path(), "validate.sh", &input(), &store, &limits);

    assert!(matches!(result, Err(RunError::Timeout)));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn runaway_output_is_cut_off_and_reported() {
    let script_content = "\
#!/bin/sh
head -c 100000 /dev/zero | tr '\\0' a
";
    let (dir, store) = project_with_script(script_content);
    let limits = Limits {
        max_output_bytes: 1000,
        ..Limits::default()
    };

    let result = run_script(dir.path(), "validate.sh", &input(), &store, &limits);

    assert!(matches!(result, Err(RunError::OutputTooLarge)));
}

#[test]
fn the_script_runs_with_a_minimal_environment() {
    let script_content = "\
#!/bin/sh
echo \"{\\\"valid\\\": false, \\\"errors\\\": [{\\\"message\\\": \\\"[$CARGO_PKG_NAME]\\\"}]}\"
";

    let output = run(script_content).unwrap();

    assert_eq!(output.errors[0].message, "[]");
}

#[test]
fn the_script_runs_in_the_project_root() {
    let script_content = "\
#!/bin/sh
echo \"{\\\"valid\\\": false, \\\"errors\\\": [{\\\"message\\\": \\\"$(pwd)\\\"}]}\"
";
    let (dir, store) = project_with_script(script_content);

    let output = run_script(
        dir.path(),
        "validate.sh",
        &input(),
        &store,
        &Limits::default(),
    )
    .unwrap();

    let expected = dir.path().canonicalize().unwrap();
    assert_eq!(output.errors[0].message, expected.to_str().unwrap());
}
