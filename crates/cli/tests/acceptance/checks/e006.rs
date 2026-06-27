use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::tempdir;

fn run_check(cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agile"))
        .arg("check")
        .current_dir(cwd)
        .output()
        .expect("failed to spawn `agile check`")
}

#[test]
fn flags_invalid_box_character() {
    let dir = tempdir().unwrap();
    // `?` is not a valid status character
    fs::write(dir.path().join("a.agile.md"), "- [?] task title\n").unwrap();

    let out = run_check(dir.path());

    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("E006"), "stdout: {stdout:?}");
}

#[test]
fn does_not_flag_valid_box_characters() {
    let dir = tempdir().unwrap();
    let content = "\
- [ ] pending
- [x] done
- [-] cancelled
";
    fs::write(dir.path().join("a.agile.md"), content).unwrap();

    let out = run_check(dir.path());

    assert!(
        out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
