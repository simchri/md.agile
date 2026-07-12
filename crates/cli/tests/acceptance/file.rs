use crate::helpers::run_agile;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git command failed to start");
    assert!(status.success(), "git {args:?} failed");
}

#[test]
fn file_prints_agile_md_files() {
    let dir = tempdir().unwrap();
    let mut file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();
    file_content = "\
not a task file
";
    fs::write(dir.path().join("README.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["file"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("tasks.agile.md"), "stdout: {stdout:?}");
    assert!(
        !stdout.contains("README.md"),
        "non-agile file should be excluded: {stdout:?}"
    );
}

#[test]
fn file_format_is_filename_then_full_path() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task
";
    fs::write(dir.path().join("my.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["file"]);

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
fn file_with_next_limit() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    let file_content = "\
- [ ] task
";
    fs::write(dir.path().join("a.agile.md"), file_content).unwrap();
    fs::write(sub.join("b.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["file", "-n", "1"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "expected exactly 1 line: {stdout:?}"
    );
}

#[test]
fn file_respects_gitignore() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);

    // Ignore the "scratch" directory and anything under it.
    let gitignore_content = "\
scratch/
";
    fs::write(dir.path().join(".gitignore"), gitignore_content).unwrap();

    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tracked.agile.md"), file_content).unwrap();

    let scratch = dir.path().join("scratch");
    fs::create_dir(&scratch).unwrap();
    fs::write(scratch.join("ignored.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["file"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("tracked.agile.md"),
        "tracked file should be listed: {stdout:?}"
    );
    assert!(
        !stdout.contains("ignored.agile.md"),
        "gitignored file should be excluded: {stdout:?}"
    );
}

#[test]
fn file_empty_project_exits_zero_with_no_output() {
    let dir = tempdir().unwrap();

    let out = run_agile(dir.path(), &["file"]);

    assert!(out.status.success());
    assert!(
        out.stdout.is_empty(),
        "expected no output: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn files_is_an_alias_for_file() {
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] a task
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    let out = run_agile(dir.path(), &["files"]);

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("tasks.agile.md"), "stdout: {stdout:?}");
}
