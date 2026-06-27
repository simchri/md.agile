use crate::helpers::run_agile;
use std::fs;
use tempfile::tempdir;

#[test]
fn list_files_prints_agile_md_files() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("tasks.agile.md"),
        "\
- [ ] a task
",
    )
    .unwrap();
    fs::write(
        dir.path().join("README.md"),
        "\
not a task file
",
    )
    .unwrap();

    let out = run_agile(dir.path(), &["list", "files"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("tasks.agile.md"), "stdout: {stdout:?}");
    assert!(
        !stdout.contains("README.md"),
        "non-agile file should be excluded: {stdout:?}"
    );
}

#[test]
fn list_files_format_is_filename_then_full_path() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("my.agile.md"),
        "\
- [ ] task
",
    )
    .unwrap();

    let out = run_agile(dir.path(), &["list", "files"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Format: "<filename>  <full-path>\n"
    let line = stdout.lines().next().expect("expected at least one line");
    assert!(
        line.starts_with("my.agile.md"),
        "line should start with filename: {line:?}"
    );
    assert!(
        line.contains("  "),
        "should have double-space separator: {line:?}"
    );
    assert!(
        line.ends_with("my.agile.md"),
        "line should end with full path: {line:?}"
    );
}

#[test]
fn list_files_with_next_limit() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    fs::write(
        dir.path().join("a.agile.md"),
        "\
- [ ] task
",
    )
    .unwrap();
    fs::write(
        sub.join("b.agile.md"),
        "\
- [ ] task
",
    )
    .unwrap();

    let out = run_agile(dir.path(), &["list", "files", "-n", "1"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "expected exactly 1 line: {stdout:?}"
    );
}

#[test]
fn list_files_empty_project_exits_zero_with_no_output() {
    let dir = tempdir().unwrap();

    let out = run_agile(dir.path(), &["list", "files"]);

    assert!(out.status.success());
    assert!(
        out.stdout.is_empty(),
        "expected no output: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}
