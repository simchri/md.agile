//! Meta-tests: enforce coding conventions across the acceptance test suite.

use std::fs;
use std::path::Path;

/// Ensures every `fs::write` call in the acceptance tests passes file content
/// through an intermediate variable using the `"\` continuation style, rather
/// than an inline string literal.
///
/// See CLAUDE.md lines 79–90 for the full rule.
#[test]
fn acceptance_tests_use_file_content_variable_not_inline_literals() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let acceptance_dir = manifest_dir.join("tests/acceptance");

    let mut violations: Vec<String> = Vec::new();
    collect_violations(&acceptance_dir, manifest_dir, &mut violations);

    assert!(
        violations.is_empty(),
        "\n\
        Convention violation: fs::write called with an inline string literal.\n\
        \n\
        File content in acceptance tests MUST use the continuation style with\n\
        an intermediate variable, NOT an inline string literal.\n\
        \n\
        WRONG:\n\
            fs::write(path, \"- [ ] task\\n\").unwrap();\n\
        \n\
        RIGHT:\n\
            let file_content = \"\\\n\
            - [ ] task\n\
            \";\n\
            fs::write(path, file_content).unwrap();\n\
        \n\
        See CLAUDE.md lines 79–90 for the full rule.\n\
        \n\
        Violations found:\n\
        {}\n",
        violations
            .iter()
            .map(|v| format!("  {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn collect_violations(dir: &Path, root: &Path, violations: &mut Vec<String>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_violations(&path, root, violations);
        } else if path.extension().map_or(false, |e| e == "rs") {
            check_file(&path, root, violations);
        }
    }
}

fn check_file(path: &Path, root: &Path, violations: &mut Vec<String>) {
    let source =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let rel = path.strip_prefix(root).unwrap_or(path);

    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue; // skip comments
        }
        if is_inline_fs_write(line) {
            violations.push(format!("{}:{}\n      {}", rel.display(), i + 1, trimmed,));
        }
    }
}

/// Returns true if the line calls `fs::write(` with a string literal as the
/// second argument instead of a variable name.
///
/// Heuristic: the first argument to `fs::write` is always a path expression
/// containing no bare commas, so the first `,` found after `fs::write(` is the
/// argument separator. If the token that follows is `"` but NOT `"\` (the
/// continuation-style opening), the content is an inline literal.
fn is_inline_fs_write(line: &str) -> bool {
    let Some(start) = line.find("fs::write(") else {
        return false;
    };
    let after = &line[start + "fs::write(".len()..];
    let Some(comma) = after.find(',') else {
        return false;
    };
    let second_arg = after[comma + 1..].trim_start();
    // `"\` is the correct continuation-style opening — not a violation.
    second_arg.starts_with('"') && !second_arg.starts_with("\"\\")
}
