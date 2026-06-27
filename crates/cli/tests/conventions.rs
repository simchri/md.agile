//! Meta-tests: enforce coding conventions across the acceptance test suite.

use std::fs;
use std::path::Path;

/// Ensures every `fs::write` call in the acceptance tests passes file content
/// through an intermediate variable, NOT an inline string literal (even when
/// using the `"\` continuation style).
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
        WRONG (inline, even with continuation style):\n\
            fs::write(path, \"- [ ] task\\n\").unwrap();\n\
            fs::write(path, \"\\\n\
            - [ ] task\n\
            \").unwrap();\n\
        \n\
        RIGHT:\n\
            let file_content = \"\\\n\
            - [ ] task\n\
            \";\n\
            fs::write(path, file_content).unwrap();\n\
        \n\
        See CLAUDE.md lines 79-90 for the full rule.\n\
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

    let needle = "fs::write(";
    let mut search_from = 0;

    while let Some(rel_pos) = source[search_from..].find(needle) {
        let call_pos = search_from + rel_pos;

        // Skip if this occurrence is inside a line comment.
        let line_start = source[..call_pos].rfind('\n').map_or(0, |p| p + 1);
        if source[line_start..call_pos].trim_start().starts_with("//") {
            search_from = call_pos + 1;
            continue;
        }

        let after_open = call_pos + needle.len();

        // Walk forward to find the first ',' not inside nested parens.
        // Handles both single-line and multi-line fs::write( calls.
        let mut depth = 0i32;
        let mut comma_pos: Option<usize> = None;
        for (i, ch) in source[after_open..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' if depth == 0 => break,
                ')' => depth -= 1,
                ',' if depth == 0 => {
                    comma_pos = Some(after_open + i);
                    break;
                }
                _ => {}
            }
        }

        if let Some(cp) = comma_pos {
            // Skip all whitespace (including newlines) after the comma.
            let second_arg = source[cp + 1..].trim_start_matches([' ', '\t', '\n', '\r']);
            // Any string literal as second arg is a violation — including the
            // `"\` continuation style. The rule requires an intermediate variable.
            if second_arg.starts_with('"') {
                let line_no = source[..call_pos].bytes().filter(|&b| b == b'\n').count() + 1;
                let line = source.lines().nth(line_no - 1).unwrap_or("").trim();
                violations.push(format!("{}:{}\n      {}", rel.display(), line_no, line));
            }
        }

        search_from = call_pos + 1;
    }
}
