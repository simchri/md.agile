use super::*;
use crate::parser;
use crate::trust::sha256_hex;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::{TempDir, tempdir};

const CONFIG: &str = "\
[Field.email]
label = \"Email\"

[Field.name]
label = \"Name\"

[Form.personal_info]
fields = [\"name\", \"email\"]
validation = \"validate.sh\"

[Properties.onboarding]
form = \"personal_info\"
";

fn config() -> Config {
    Config::from_str(CONFIG).unwrap()
}

/// Writes an executable `validate.sh`; returns the project dir and a store
/// trusting it.
fn project(script_content: &str) -> (TempDir, TrustStore) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("validate.sh");
    fs::write(&path, script_content).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let mut store = TrustStore::default();
    store.trust("validate.sh", &sha256_hex(script_content.as_bytes()));
    (dir, store)
}

fn items(file_content: &str) -> Vec<FileItem> {
    parser::parse(file_content, "tasks.agile.md".into())
}

#[test]
fn branch_form_scripts_apply_only_to_selected_branches_and_keep_trust_checks() {
    let config = Config::from_str(
        "\
[Form.base]
validation = \"base.sh\"
[Form.approval]
validation = \"approval.sh\"
[Form.rejection]
validation = \"rejection.sh\"
[Properties.review]
form = \"base\"
[Properties.review.passed]
form = \"approval\"
[Properties.review.failed]
form = \"rejection\"
",
    )
    .unwrap();
    let dir = tempdir().unwrap();
    let store = TrustStore::default();
    for name in ["base.sh", "approval.sh", "rejection.sh"] {
        fs::write(dir.path().join(name), "#!/bin/sh").unwrap();
    }
    for (outcome, expected_forms) in [
        ("...", vec!["base"]),
        ("unknown", vec!["base"]),
        ("passed", vec!["base", "approval"]),
        ("failed", vec!["base", "rejection"]),
    ] {
        let file_content = format!(
            "\
- [ ] #review:{outcome}
"
        );

        let issues = check_form_scripts(
            &items(&file_content),
            dir.path(),
            &config,
            &store,
            &Limits::default(),
        );

        let forms: Vec<_> = issues
            .iter()
            .map(|issue| {
                assert_eq!(issue.code, ErrorCode::FormScriptFailed);
                assert!(issue.message.contains("not trusted"), "{issue:?}");
                let Some(IssueData::FormScriptFailed { form, .. }) = &issue.data else {
                    panic!("expected form script failure");
                };
                form.as_str()
            })
            .collect();
        assert_eq!(forms, expected_forms, "{outcome}");
    }
}

#[test]
fn branch_form_scripts_run_once_per_active_form_with_isolated_inputs() {
    let config = Config::from_str(
        "\
[Field.base]
label = \"Base\"
[Field.branch]
label = \"Branch\"
[Form.base]
fields = [\"base\"]
validation = \"validate.sh\"
[Form.branch]
fields = [\"branch\"]
validation = \"validate.sh\"
[Properties.review]
form = \"base\"
[Properties.review.passed]
form = \"branch\"
[Properties.review.failed]
form = \"branch\"
[Properties.shared]
form = \"branch\"
",
    )
    .unwrap();
    let script_content = "\
#!/bin/sh
cat >> inputs.jsonl
printf '\\n' >> inputs.jsonl
echo '{\"valid\": true}'
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] #review:passed #shared #review:passed
  __Base:__ base answer
  __Branch:__ branch answer
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config,
        &store,
        &Limits::default(),
    );

    assert!(issues.is_empty(), "{issues:?}");
    let inputs = fs::read_to_string(dir.path().join("inputs.jsonl")).unwrap();
    let inputs: Vec<serde_json::Value> = inputs
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0]["form"], "base");
    assert_eq!(
        inputs[0]["fields"],
        serde_json::json!({"base": "base answer"})
    );
    assert_eq!(inputs[1]["form"], "branch");
    assert_eq!(
        inputs[1]["fields"],
        serde_json::json!({"branch": "branch answer"})
    );
}

#[test]
fn a_valid_form_yields_no_issues() {
    let script_content = "\
#!/bin/sh
echo '{\"valid\": true}'
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] new colleague #onboarding
  __Name:__ John
  __Email:__ john@example.com
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &store,
        &Limits::default(),
    );

    assert_eq!(issues, vec![]);
}

#[test]
fn a_field_error_is_anchored_at_the_field_line() {
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false, \"errors\": [{\"field\": \"email\", \"message\": \"not an email\"}]}'
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] new colleague #onboarding
  __Name:__ John
  __Email:__ john
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &store,
        &Limits::default(),
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::FormValidationFailed);
    assert_eq!(issues[0].location.line, 3);
    assert!(issues[0].message.contains("not an email"));
    assert_eq!(
        issues[0].data,
        Some(IssueData::FormValidationFailed {
            form: "personal_info".to_string(),
            field_key: Some("email".to_string()),
            message: "not an email".to_string(),
        })
    );
}

#[test]
fn an_error_without_a_field_is_anchored_at_the_task_line() {
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false, \"errors\": [{\"message\": \"inconsistent\"}]}'
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] other
- [ ] new colleague #onboarding
  __Name:__ John
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &store,
        &Limits::default(),
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 2);
}

#[test]
fn an_error_naming_an_absent_field_falls_back_to_the_task_line() {
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false, \"errors\": [{\"field\": \"email\", \"message\": \"required\"}]}'
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] new colleague #onboarding
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &store,
        &Limits::default(),
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 1);
}

#[test]
fn an_error_naming_a_key_unknown_to_the_form_falls_back_to_the_task_line() {
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false, \"errors\": [{\"field\": \"nickname\", \"message\": \"odd\"}]}'
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] new colleague #onboarding
  __Name:__ John
  __Email:__ john@example.com
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &store,
        &Limits::default(),
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location.line, 1);
    assert!(issues[0].message.contains("odd"));
}

#[test]
fn subtasks_are_validated_with_their_own_fields_and_line() {
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false, \"errors\": [{\"field\": \"email\", \"message\": \"not an email\"}]}'
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] onboarding project
  - [ ] new colleague #onboarding
    __Name:__ John
    __Email:__ john
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &store,
        &Limits::default(),
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::FormValidationFailed);
    assert_eq!(issues[0].location.line, 4);
}

#[test]
fn an_invalid_verdict_without_errors_still_yields_one_issue() {
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false}'
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] new colleague #onboarding
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &store,
        &Limits::default(),
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::FormValidationFailed);
}

#[test]
fn the_script_receives_only_the_forms_fields_in_json() {
    let script_content = "\
#!/bin/sh
cat > received.json
echo '{\"valid\": true}'
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] new colleague #onboarding
  __Name:__ John
  __Email:__ john@example.com
  __Unrelated:__ x
";

    check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &store,
        &Limits::default(),
    );

    let received: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("received.json")).unwrap())
            .unwrap();
    assert_eq!(received["form"], "personal_info");
    assert_eq!(received["task"]["title"], "new colleague #onboarding");
    assert_eq!(received["task"]["line"], 1);
    assert_eq!(
        received["fields"],
        serde_json::json!({"name": "John", "email": "john@example.com"})
    );
}

#[test]
fn an_untrusted_script_yields_e031_with_trust_instructions() {
    let script_content = "\
#!/bin/sh
echo '{\"valid\": true}'
";
    let (dir, _) = project(script_content);
    let file_content = "\
- [ ] new colleague #onboarding
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &TrustStore::default(),
        &Limits::default(),
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::FormScriptFailed);
    assert!(issues[0].message.contains("validate.sh"));
    let help = issues[0].help.as_deref().unwrap();
    assert!(help.contains("agile trust --yes"), "{help}");
}

#[test]
fn a_crashing_script_yields_e031() {
    let script_content = "\
#!/bin/sh
echo boom >&2
exit 3
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] new colleague #onboarding
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &store,
        &Limits::default(),
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::FormScriptFailed);
    assert!(issues[0].message.contains("boom"), "{}", issues[0].message);
}

#[test]
fn a_missing_script_yields_e031() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] new colleague #onboarding
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &config(),
        &TrustStore::default(),
        &Limits::default(),
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, ErrorCode::FormScriptFailed);
}

#[test]
fn forms_without_a_validation_script_are_not_run() {
    let config_content = "\
[Field.email]
label = \"Email\"

[Form.personal_info]
fields = [\"email\"]

[Properties.onboarding]
form = \"personal_info\"
";
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] new colleague #onboarding
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &Config::from_str(config_content).unwrap(),
        &TrustStore::default(),
        &Limits::default(),
    );

    assert_eq!(issues, vec![]);
}

#[test]
fn the_script_runs_once_per_task_even_if_two_properties_share_the_form() {
    let config_content = "\
[Field.email]
label = \"Email\"

[Form.personal_info]
fields = [\"email\"]
validation = \"validate.sh\"

[Properties.onboarding]
form = \"personal_info\"

[Properties.hr]
form = \"personal_info\"
";
    let script_content = "\
#!/bin/sh
echo '{\"valid\": false, \"errors\": [{\"message\": \"bad\"}]}'
";
    let (dir, store) = project(script_content);
    let file_content = "\
- [ ] new colleague #onboarding #hr
";

    let issues = check_form_scripts(
        &items(file_content),
        dir.path(),
        &Config::from_str(config_content).unwrap(),
        &store,
        &Limits::default(),
    );

    assert_eq!(issues.len(), 1);
}
