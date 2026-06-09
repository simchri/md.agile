/// Pure helpers for the `textDocument/definition` handler.
///
/// Both functions are free of I/O and async so they can be unit-tested
/// without spinning up the full LSP server.
use crate::parser::SpecialMarker;

// ── Shared cursor helper ──────────────────────────────────────────────────────

/// Scan the source line at `line` and return the bare name of the marker
/// token under the cursor, or `None`.
///
/// A marker token is any run of non-whitespace characters that starts with
/// `sigil` (`'#'` for properties, `'@'` for assignments). The returned string
/// is everything **after** the sigil; the caller is responsible for any
/// further normalisation (e.g. stripping branch suffixes for properties).
fn token_name_at_position(text: &str, line: u32, character: u32, sigil: char) -> Option<String> {
    let line_text = text.lines().nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let char_idx = character as usize;

    if char_idx > chars.len() {
        return None;
    }

    // Walk left to find the start of the whitespace-delimited token.
    let mut start = char_idx;
    while start > 0 && !chars[start - 1].is_ascii_whitespace() {
        start -= 1;
    }

    // The token must begin with the expected sigil.
    if chars.get(start) != Some(&sigil) {
        return None;
    }

    // Walk right to find the end of the token.
    let mut end = start + 1;
    while end < chars.len() && !chars[end].is_ascii_whitespace() {
        end += 1;
    }

    // The cursor must lie within the token span (start..end).
    if char_idx < start || char_idx >= end {
        return None;
    }

    // Everything after the leading sigil.
    let raw: String = chars[start + 1..end].iter().collect();
    if raw.is_empty() { None } else { Some(raw) }
}

// ── Properties ────────────────────────────────────────────────────────────────

/// Given the full text of an open `.agile.md` document and a cursor
/// position, return the canonical property name that the cursor sits on,
/// or `None` if the cursor is not on a `#property` token.
///
/// Name normalisation mirrors `parse_hash_token` exactly:
/// - `#review...`   → `"review"` (BranchPending: strip `...` suffix)
/// - `#review:done` → `"review"` (BranchResolved: keep the part before `:`)
/// - `#feat:`       → `"feat"`   (trailing `:;,.` stripped)
/// - `#feat`        → `"feat"`   (Full form, no change)
/// - `#OPT` / `#MILESTONE` / `#MDAGILE` → `None` (special markers)
pub fn property_name_at_position(text: &str, line: u32, character: u32) -> Option<String> {
    let raw = token_name_at_position(text, line, character, '#')?;
    normalize_property_name(&raw)
}

/// Normalize a raw token body (the part after `#`) into a property name,
/// using the same rules as `parse_hash_token` in the parser.
/// Returns `None` for special markers and empty / unrecognised tokens.
fn normalize_property_name(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }

    // Reject special ALL-CAPS markers.
    if SpecialMarker::from_name(raw, 0).is_some() {
        return None;
    }

    // BranchPending: `review...`
    if let Some(base) = raw.strip_suffix("...") {
        if !base.is_empty() {
            return Some(base.to_string());
        }
    }

    // BranchResolved: `review:passed`
    if let Some(pos) = raw.find(':') {
        let base = &raw[..pos];
        let branch = &raw[pos + 1..];
        if !base.is_empty() && !branch.is_empty() {
            return Some(base.to_string());
        }
    }

    // Full form (possibly with trailing punctuation): `feat` or `feat:`
    let clean = raw.trim_end_matches(|c: char| ":;,.".contains(c));
    if clean.is_empty() {
        return None;
    }
    Some(clean.to_string())
}

/// Scan `config_text` (the contents of `mdagile.toml`) and return the
/// **0-based** line number where `name` is declared, or `None`.
///
/// Handles:
/// - Dotted table header:  `[Properties.name]`
/// - Flat key under `[Properties]`:  `name = ...`
/// - Inline TOML comments after the section header: `[Properties.name] # ok`
pub fn find_property_line_in_config(config_text: &str, name: &str) -> Option<u32> {
    let dotted_header = format!("[Properties.{}]", name);
    let mut in_properties_section = false;

    for (idx, line) in config_text.lines().enumerate() {
        // Strip inline TOML comment (` # ...`) for comparison purposes.
        let trimmed = line.split(" #").next().unwrap_or(line).trim();

        // Dotted table header: [Properties.name]
        if trimmed == dotted_header {
            return Some(idx as u32);
        }

        // Track flat [Properties] section.
        if trimmed == "[Properties]" {
            in_properties_section = true;
            continue;
        }

        // Any other section header ends the flat Properties section.
        if trimmed.starts_with('[') {
            in_properties_section = false;
            continue;
        }

        // Inside [Properties], look for `name = ...`
        if in_properties_section {
            let key = trimmed.split('=').next().unwrap_or("").trim();
            if key == name {
                return Some(idx as u32);
            }
        }
    }

    None
}

// ── Assignments ───────────────────────────────────────────────────────────────

/// Given the full text of an open `.agile.md` document and a cursor
/// position, return the assignment name that the cursor sits on,
/// or `None` if the cursor is not on an `@assignment` token.
///
/// Assignment names have no special forms — `@alice` → `"alice"`.
pub fn assignment_name_at_position(text: &str, line: u32, character: u32) -> Option<String> {
    token_name_at_position(text, line, character, '@')
}

/// Scan `config_text` (the contents of `mdagile.toml`) and return the
/// **0-based** line number where `name` is declared as a user or group,
/// or `None`.
///
/// Searches `[Users.name]` and `[Groups.name]` dotted table headers.
/// Inline TOML comments are stripped before comparison.
pub fn find_assignment_line_in_config(config_text: &str, name: &str) -> Option<u32> {
    for (idx, line) in config_text.lines().enumerate() {
        let trimmed = line.split(" #").next().unwrap_or(line).trim();
        if trimmed == format!("[Users.{}]", name) || trimmed == format!("[Groups.{}]", name) {
            return Some(idx as u32);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── property_name_at_position ────────────────────────────────────────────

    #[test]
    fn returns_name_for_full_property_cursor_on_hash() {
        let doc = "- [ ] task #feat\n";
        assert_eq!(
            property_name_at_position(doc, 0, 11),
            Some("feat".to_string())
        );
    }

    #[test]
    fn returns_name_for_full_property_cursor_on_last_char() {
        let doc = "- [ ] task #feat\n";
        // 'feat' occupies chars 12-15 (0-based); cursor on 't' at 15.
        assert_eq!(
            property_name_at_position(doc, 0, 15),
            Some("feat".to_string())
        );
    }

    #[test]
    fn returns_base_name_for_branch_pending() {
        // #review... — cursor somewhere inside the token
        let doc = "- [ ] task #review...\n";
        assert_eq!(
            property_name_at_position(doc, 0, 13),
            Some("review".to_string())
        );
    }

    #[test]
    fn returns_base_name_for_branch_resolved() {
        // #review:passed — cursor inside the token
        let doc = "- [ ] task #review:passed\n";
        assert_eq!(
            property_name_at_position(doc, 0, 14),
            Some("review".to_string())
        );
    }

    #[test]
    fn returns_name_stripped_of_trailing_punctuation() {
        // #feat: — trailing colon is stripped
        let doc = "- [ ] task #feat:\n";
        assert_eq!(
            property_name_at_position(doc, 0, 12),
            Some("feat".to_string())
        );
    }

    #[test]
    fn returns_none_when_cursor_on_plain_text() {
        let doc = "- [ ] task #feat\n";
        // Cursor on 't' in "task" (position 8)
        assert_eq!(property_name_at_position(doc, 0, 8), None);
    }

    #[test]
    fn returns_none_for_opt_special_marker() {
        let doc = "- [ ] task #OPT\n";
        assert_eq!(property_name_at_position(doc, 0, 12), None);
    }

    #[test]
    fn returns_none_for_milestone_special_marker() {
        let doc = "- [ ] task #MILESTONE\n";
        assert_eq!(property_name_at_position(doc, 0, 13), None);
    }

    #[test]
    fn returns_none_when_cursor_past_end_of_line() {
        let doc = "- [ ] task\n";
        assert_eq!(property_name_at_position(doc, 0, 100), None);
    }

    #[test]
    fn returns_none_for_line_out_of_range() {
        let doc = "- [ ] task\n";
        assert_eq!(property_name_at_position(doc, 5, 0), None);
    }

    #[test]
    fn returns_name_for_second_property_on_same_line() {
        // Multiple markers: cursor on #bug
        let doc = "- [ ] task #feat #bug\n";
        // '#bug' starts at column 17
        assert_eq!(
            property_name_at_position(doc, 0, 18),
            Some("bug".to_string())
        );
    }

    #[test]
    fn returns_none_when_cursor_on_at_marker() {
        let doc = "- [ ] task @alice\n";
        // '@alice' doesn't start with '#'
        assert_eq!(property_name_at_position(doc, 0, 12), None);
    }

    // ── assignment_name_at_position ──────────────────────────────────────────

    #[test]
    fn assignment_returns_name_cursor_on_at_sign() {
        let doc = "- [ ] task @alice\n";
        assert_eq!(
            assignment_name_at_position(doc, 0, 11),
            Some("alice".to_string())
        );
    }

    #[test]
    fn assignment_returns_name_cursor_on_last_char() {
        let doc = "- [ ] task @alice\n";
        // '@alice' is at chars 11-16; cursor on 'e' at 16.
        assert_eq!(
            assignment_name_at_position(doc, 0, 16),
            Some("alice".to_string())
        );
    }

    #[test]
    fn assignment_returns_none_cursor_on_plain_text() {
        let doc = "- [ ] task @alice\n";
        // Cursor on 't' in "task" (position 6)
        assert_eq!(assignment_name_at_position(doc, 0, 6), None);
    }

    #[test]
    fn assignment_returns_none_cursor_on_hash_marker() {
        let doc = "- [ ] task #feat\n";
        assert_eq!(assignment_name_at_position(doc, 0, 12), None);
    }

    #[test]
    fn assignment_returns_none_for_line_out_of_range() {
        let doc = "- [ ] task @alice\n";
        assert_eq!(assignment_name_at_position(doc, 5, 0), None);
    }

    #[test]
    fn assignment_returns_name_for_second_assignment_on_line() {
        let doc = "- [ ] task @alice @bob\n";
        // '@bob' starts at column 18
        assert_eq!(
            assignment_name_at_position(doc, 0, 19),
            Some("bob".to_string())
        );
    }

    // ── find_assignment_line_in_config ───────────────────────────────────────

    #[test]
    fn finds_user_dotted_header() {
        let config = "\
[Users.alice]
[Users.bob]
";
        assert_eq!(find_assignment_line_in_config(config, "alice"), Some(0));
        assert_eq!(find_assignment_line_in_config(config, "bob"), Some(1));
    }

    #[test]
    fn finds_group_dotted_header() {
        let config = "\
[Groups.backend]
[Groups.frontend]
";
        assert_eq!(find_assignment_line_in_config(config, "backend"), Some(0));
        assert_eq!(find_assignment_line_in_config(config, "frontend"), Some(1));
    }

    #[test]
    fn finds_user_before_group_with_same_name() {
        let config = "\
[Groups.alice]
[Users.alice]
";
        // Returns the first match (Groups.alice at line 0).
        assert_eq!(find_assignment_line_in_config(config, "alice"), Some(0));
    }

    #[test]
    fn find_assignment_returns_none_when_absent() {
        let config = "[Users.alice]\n";
        assert_eq!(find_assignment_line_in_config(config, "bob"), None);
    }

    #[test]
    fn find_assignment_handles_inline_comment() {
        let config = "[Users.alice] # the team lead\n";
        assert_eq!(find_assignment_line_in_config(config, "alice"), Some(0));
    }

    #[test]
    fn find_assignment_does_not_confuse_prefix_match() {
        // [Users.ali] must NOT match a search for "alice"
        let config = "\
[Users.ali]
[Users.alice]
";
        assert_eq!(find_assignment_line_in_config(config, "alice"), Some(1));
    }

    // ── find_property_line_in_config ─────────────────────────────────────────

    #[test]
    fn finds_dotted_table_header() {
        let config = "\
[Properties.feat]
[Properties.bug]
";
        assert_eq!(find_property_line_in_config(config, "feat"), Some(0));
        assert_eq!(find_property_line_in_config(config, "bug"), Some(1));
    }

    #[test]
    fn finds_key_under_flat_properties_section() {
        let config = "\
[Properties]
feat = {}
bug = {}
";
        assert_eq!(find_property_line_in_config(config, "feat"), Some(1));
        assert_eq!(find_property_line_in_config(config, "bug"), Some(2));
    }

    #[test]
    fn returns_none_when_property_absent() {
        let config = "\
[Properties.feat]
";
        assert_eq!(find_property_line_in_config(config, "review"), None);
    }

    #[test]
    fn ignores_other_sections() {
        let config = "\
[Other]
feat = {}
[Properties.feat]
";
        // The `feat = {}` under [Other] must NOT match; only line 2 matches.
        assert_eq!(find_property_line_in_config(config, "feat"), Some(2));
    }

    #[test]
    fn handles_inline_comment_on_dotted_header() {
        let config = "\
[Properties.feat] # the feature tracker
";
        assert_eq!(find_property_line_in_config(config, "feat"), Some(0));
    }

    #[test]
    fn does_not_confuse_prefix_match() {
        // [Properties.feature] must NOT match a search for "feat"
        let config = "\
[Properties.feature]
[Properties.feat]
";
        assert_eq!(find_property_line_in_config(config, "feat"), Some(1));
    }
}
