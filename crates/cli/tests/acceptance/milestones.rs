use crate::helpers::run_agile;
use std::fs;
use tempfile::tempdir;

#[test]
fn milestones_lists_future_milestones_with_weighted_counts_by_default() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task a
- [x] task b
- [ ] task c
#MILESTONE: alpha
- [ ] task d
  - [ ] subtask d1
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let out = run_agile(dir.path(), &["milestones"]);

    // Assert
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let expected = "\
1 alpha 2 /   3 66%
2 beta  0 / 1.5  0%
";
    assert_eq!(stdout, expected);
}

#[test]
fn milestones_count_flag_shows_top_level_task_counts_instead_of_weight() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task a
  - [ ] subtask a1
- [ ] task b
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let out = run_agile(dir.path(), &["milestones", "--count"]);

    // Assert
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout, "1 alpha 1 / 2 50%\n");
}

#[test]
fn milestones_skips_already_reached_milestones() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task a
#MILESTONE: alpha
- [ ] task b
#MILESTONE: beta
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let out = run_agile(dir.path(), &["milestones"]);

    // Assert
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("alpha"), "stdout: {stdout:?}");
    assert!(stdout.contains("1 beta"), "stdout: {stdout:?}");
}

#[test]
fn milestones_prints_no_milestones_when_none_are_future() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let out = run_agile(dir.path(), &["milestones"]);

    // Assert
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout, "no milestones\n");
}

#[test]
fn milestones_next_rank_shows_detail_breakdown() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [x] task a
- [x] task b
- [x] task c
- [ ] task d
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let out = run_agile(dir.path(), &["milestones", "--next", "1"]);

    // Assert
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let expected = "\
milestone: alpha
tasks since last milestone: 4
tasks to do: 1
tasks done: 3
tasks percentage done: 75%
weight to do: 1
weight done: 3
weight percentage done: 75%
";
    assert_eq!(stdout, expected);
}

#[test]
fn milestones_next_rank_out_of_range_errors_with_nonzero_exit() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let out = run_agile(dir.path(), &["milestones", "--next", "5"]);

    // Assert
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("milestone rank 5 does not exist"),
        "stderr: {stderr:?}"
    );
}

#[test]
fn milestone_alias_behaves_identically_to_milestones() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let out = run_agile(dir.path(), &["milestone"]);

    // Assert
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout, "1 alpha 0 / 1 0%\n");
}

#[test]
fn milestones_truncates_long_names_with_ellipsis() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: this milestone name is way too long to display in full
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let out = run_agile(dir.path(), &["milestones"]);

    // Assert
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout,
        "1 this milestone name is way too long to … 0 / 1 0%\n"
    );
}

#[test]
fn milestones_count_conflicts_with_next() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_content = "\
- [ ] task a
#MILESTONE: alpha
";
    fs::write(dir.path().join("tasks.agile.md"), file_content).unwrap();

    // Act
    let out = run_agile(dir.path(), &["milestones", "--next", "1", "--count"]);

    // Assert
    assert!(!out.status.success());
}
