//! LSP Semantic Tokens for `.agile.md` files.
//!
//! Highlights three kinds of markers:
//!
//! - Built-in special markers as `keyword` tokens:
//!   - `#OPT` appears as a marker on task/subtask lines.
//!   - `#MILESTONE: Name` appears as a standalone header line
//!     (`FileItem::Milestone`) — the `#MILESTONE` keyword is highlighted
//!     at column 0 of that line.
//! - User-defined `#property` markers as `property` tokens. For branch-form
//!   properties (`#review...`, `#review:passed`) only the base name is
//!   highlighted.
//! - `@user`/`@group` assignment markers as `parameter` tokens.

use crate::parser::{FileItem, Marker, SpecialMarkerKind, Subtask, TASK_LINE_PREFIX_LEN};
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenType};

// ── Legend ────────────────────────────────────────────────────────────────────

/// Token type legend to advertise in `initialize`.
/// Index positions correspond to `SemanticToken.token_type` values.
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,   // 0 — #OPT / #MILESTONE / #MDAGILE
    SemanticTokenType::PROPERTY,  // 1 — user-defined #property markers
    SemanticTokenType::PARAMETER, // 2 — @user/@group assignment markers
];

const KEYWORD: u32 = 0;
const PROPERTY: u32 = 1;
const PARAMETER: u32 = 2;

/// Length of the `#MILESTONE` token (sigil + keyword name).
const MILESTONE_TOKEN_LEN: u32 = "#MILESTONE".len() as u32;

// ── Public entry point ────────────────────────────────────────────────────────

/// Build the semantic token list for a parsed document.
///
/// Returns tokens delta-encoded in ascending line/character order as required
/// by the LSP specification.
pub fn build_tokens(items: &[FileItem]) -> Vec<SemanticToken> {
    let mut raw: Vec<RawToken> = Vec::new();

    for item in items {
        match item {
            FileItem::Task(task) => {
                collect_markers(&task.markers, task.location.line, task.indent, &mut raw);
                collect_subtasks(&task.children, &mut raw);
            }
            FileItem::Milestone(m) => {
                // Highlight the `#MILESTONE` keyword on the header line.
                // The token always starts at column 0 of the (trimmed) line.
                raw.push(RawToken {
                    line: (m.line - 1) as u32,
                    character: 0,
                    length: MILESTONE_TOKEN_LEN,
                    token_type: KEYWORD,
                });
            }
        }
    }

    raw.sort_unstable_by_key(|t| (t.line, t.character));
    encode_delta(raw)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

struct RawToken {
    line: u32,
    character: u32,
    length: u32,
    token_type: u32,
}

fn collect_subtasks(subtasks: &[Subtask], raw: &mut Vec<RawToken>) {
    for sub in subtasks {
        collect_markers(&sub.markers, sub.location.line, sub.indent, raw);
        collect_subtasks(&sub.children, raw);
    }
}

/// Collect semantic tokens for all markers on a single task/subtask line.
///
/// `location_line` is 1-based; `indent` is the number of leading spaces.
/// Each marker stores the 1-based column of `#`/`@` within the title text
/// (after the `"  - [ ] "` prefix). The 0-based character in the full source
/// line is therefore `indent + (TASK_LINE_PREFIX_LEN - 1) + column`.
///
/// Each marker is mapped to `(column, name, token_type)` and pushed once;
/// for branch-form properties (`#review...`, `#review:passed`) only the base
/// name is highlighted, not the suffix.
fn collect_markers(
    markers: &[Marker],
    location_line: usize,
    indent: usize,
    raw: &mut Vec<RawToken>,
) {
    let line = (location_line - 1) as u32;
    // Convert 1-based title column to 0-based source-line character:
    // indent + (TASK_LINE_PREFIX_LEN - 1) + column
    let char_of = |column: usize| (indent + TASK_LINE_PREFIX_LEN - 1 + column) as u32;
    for marker in markers {
        let (column, name, token_type): (usize, &str, u32) = match marker {
            Marker::Special(special) => {
                if matches!(
                    special.kind,
                    SpecialMarkerKind::Milestone | SpecialMarkerKind::MdAgile
                ) {
                    continue;
                }
                (special.column, special.as_str(), KEYWORD)
            }
            Marker::Property(prop) => (prop.column, prop.name.as_str(), PROPERTY),
            Marker::Assignment(a) => (a.column, a.name.as_str(), PARAMETER),
        };
        raw.push(RawToken {
            line,
            character: char_of(column),
            length: (1 + name.len()) as u32, // sigil ('#' or '@') + name
            token_type,
        });
    }
}

/// Convert absolute (line, character) positions into the delta-encoded form
/// required by the LSP spec.
fn encode_delta(sorted: Vec<RawToken>) -> Vec<SemanticToken> {
    let mut tokens = Vec::with_capacity(sorted.len());
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for t in sorted {
        let delta_line = t.line - prev_line;
        let delta_start = if delta_line == 0 {
            t.character - prev_start
        } else {
            t.character
        };
        tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length: t.length,
            token_type: t.token_type,
            token_modifiers_bitset: 0,
        });
        prev_line = t.line;
        prev_start = t.character;
    }
    tokens
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use std::path::PathBuf;

    fn p(input: &str) -> Vec<FileItem> {
        parse(input, PathBuf::from("test.agile.md"))
    }

    /// Guard that TOKEN_TYPES order matches the KEYWORD/PROPERTY/PARAMETER
    /// index constants. If these ever drift apart, all highlighting breaks
    /// silently — this test catches it immediately.
    #[test]
    fn token_type_indices_match_legend() {
        assert_eq!(TOKEN_TYPES[KEYWORD as usize], SemanticTokenType::KEYWORD);
        assert_eq!(TOKEN_TYPES[PROPERTY as usize], SemanticTokenType::PROPERTY);
        assert_eq!(
            TOKEN_TYPES[PARAMETER as usize],
            SemanticTokenType::PARAMETER
        );
    }

    // ── #OPT on a subtask ─────────────────────────────────────────────────────

    #[test]
    fn opt_marker_as_first_token_in_title() {
        // "- [ ] parent\n  - [ ] #OPT optional thing\n"
        // Subtask title text: "#OPT optional thing"  → #OPT at col 1 (1-based)
        // Full line (indent=2): "  - [ ] #OPT optional thing"
        //   → char = 2 + 5 + 1 = 8
        let items = p("\
- [ ] parent
  - [ ] #OPT optional thing
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        let t = &tokens[0];
        assert_eq!(t.delta_line, 1); // line 1 (0-based), first token → delta = 1
        assert_eq!(t.delta_start, 8);
        assert_eq!(t.length, 4); // "#OPT"
        assert_eq!(t.token_type, KEYWORD);
        assert_eq!(t.token_modifiers_bitset, 0);
    }

    #[test]
    fn opt_marker_after_title_words() {
        // "- [ ] parent\n  - [ ] optional thing #OPT\n"
        // Subtask title text: "optional thing #OPT"
        // "#OPT" token is at position 15 (0-based) in title → col = 16 (1-based)
        // Full line (indent=2): char = 2 + 5 + 16 = 23
        let items = p("\
- [ ] parent
  - [ ] optional thing #OPT
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].delta_start, 23);
        assert_eq!(tokens[0].length, 4);
    }

    // ── #MILESTONE header line ────────────────────────────────────────────────

    #[test]
    fn milestone_header_line_emits_token() {
        // "#MILESTONE: Sprint 1" → FileItem::Milestone on line 1
        let items = p("#MILESTONE: Sprint 1\n");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        let t = &tokens[0];
        assert_eq!(t.delta_line, 0); // line 0 (0-based), first token
        assert_eq!(t.delta_start, 0); // starts at column 0
        assert_eq!(t.length, 10); // "#MILESTONE"
        assert_eq!(t.token_type, KEYWORD);
    }

    #[test]
    fn milestone_header_between_tasks_correct_line() {
        let items = p("\
- [ ] task one
#MILESTONE: Sprint 2
- [ ] task two
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        // The milestone header is on line 2 (1-based) → delta_line = 1 (0-based)
        assert_eq!(tokens[0].delta_line, 1);
        assert_eq!(tokens[0].delta_start, 0);
        assert_eq!(tokens[0].length, 10);
    }

    // ── #MILESTONE edge cases ────────────────────────────────────────────────

    #[test]
    fn milestone_colon_form_in_task_is_property_not_keyword() {
        // Inline `#MILESTONE: ...` on a task line is NOT the milestone keyword.
        // The trailing colon prevents an exact special-keyword match, so the
        // parser demotes it to a property named "MILESTONE" — highlighted as a
        // `property`, never as a `keyword`.
        let items = p("\
- [ ] task one #MILESTONE: Sprint 2
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, PROPERTY);
        assert_eq!(tokens[0].length, 10); // "#MILESTONE"
    }

    #[test]
    fn milestone_in_subtasks_not_semantic_token() {
        let items = p("\
- [ ] parent task
  - [ ] #MILESTONE not a highlighted as keyword here 
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 0);
    }

    // ── #MDAGILE ─────────────────────────────────────────────────────────────

    #[test]
    #[ignore = "Config keys not yet fully implemented, more work to do, conecptually and in parser"]
    fn mdagile_marker_on_free_line() {
        let items = p("\
Something something

#MDAGILE.something.something

- [ ] some task
  - [ ] and subtask
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].delta_start, 1);
        assert_eq!(tokens[0].length, 8); // "#MDAGILE"
    }

    #[test]
    fn mdagile_colon_form_in_task_is_property_not_keyword() {
        // Inline `#MDAGILE: ...` on a task line is NOT the mdagile keyword.
        // The trailing colon prevents an exact special-keyword match, so the
        // parser demotes it to a property named "MDAGILE" — highlighted as a
        // `property`, never as a `keyword`.
        let items = p("\
- [ ] task one #MDAGILE: Not highlighted here
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, PROPERTY);
        assert_eq!(tokens[0].length, 8); // "#MDAGILE"
    }

    #[test]
    fn mdagile_in_subtasks_not_semantic_token() {
        let items = p("\
- [ ] parent task
  - [ ] #MDAGILE not a highlighted as keyword here 
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 0);
    }
    // ── Multiple markers / delta encoding ────────────────────────────────────

    #[test]
    fn two_markers_on_same_line_have_correct_deltas() {
        // Subtask: "  - [ ] #OPT thing #OPT"
        // title: "#OPT thing #OPT"
        // First #OPT at col 1, char = 2+5+1 = 8, len 4
        // Second #OPT: "thing #OPT" — #OPT starts at title pos 11 → col 12
        // char for second #OPT = 2 + 5 + 12 = 19
        let items = p("\
- [ ] parent
  - [ ] #OPT thing #OPT
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 2);
        // First token: #OPT
        assert_eq!(tokens[0].delta_line, 1);
        assert_eq!(tokens[0].delta_start, 8);
        assert_eq!(tokens[0].length, 4);
        // Second token: #OPT — same line, delta_start relative to prev
        assert_eq!(tokens[1].delta_line, 0);
        assert_eq!(tokens[1].delta_start, 19 - 8);
        assert_eq!(tokens[1].length, 4);
    }

    #[test]
    fn markers_on_different_lines_have_correct_delta_lines() {
        let items = p("\
- [ ] parent
  - [ ] #OPT first
  - [ ] #OPT second
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].delta_line, 1); // line 1 (0-based)
        assert_eq!(tokens[1].delta_line, 1); // one line after previous
        assert_eq!(tokens[1].delta_start, 8); // absolute start (new line)
    }

    #[test]
    fn no_tokens_for_plain_task() {
        let items = p("- [ ] a plain task with no markers\n");
        assert!(build_tokens(&items).is_empty());
    }

    #[test]
    fn property_marker_emits_property_token() {
        // #feature is a property, not a special marker.
        // Full line: "- [ ] task #feature" → '#' at 0-based char 11.
        let items = p("- [ ] task #feature\n");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        let t = &tokens[0];
        assert_eq!(t.delta_line, 0);
        assert_eq!(t.delta_start, 11);
        assert_eq!(t.length, 8); // "#feature"
        assert_eq!(t.token_type, PROPERTY);
        assert_eq!(t.token_modifiers_bitset, 0);
    }

    #[test]
    fn branch_pending_property_highlights_base_name_only() {
        // "#review..." → base name "review"; suffix "..." is not highlighted.
        // Full line: "- [ ] task #review..." → '#' at char 11, length 1+6 = 7.
        let items = p("- [ ] task #review...\n");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].delta_start, 11);
        assert_eq!(tokens[0].length, 7); // "#review"
        assert_eq!(tokens[0].token_type, PROPERTY);
    }

    #[test]
    fn branch_resolved_property_highlights_base_name_only() {
        // "#review:passed" → base name "review"; ":passed" is not highlighted.
        let items = p("- [ ] task #review:passed\n");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].delta_start, 11);
        assert_eq!(tokens[0].length, 7); // "#review"
        assert_eq!(tokens[0].token_type, PROPERTY);
    }

    #[test]
    fn property_and_special_marker_get_distinct_token_types() {
        // Subtask "  - [ ] #OPT #feature": #OPT keyword, #feature property.
        let items = p("\
- [ ] parent
  - [ ] #OPT #feature
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 2);
        // #OPT at indent=2: char = 2 + 5 + 1 = 8
        assert_eq!(tokens[0].delta_start, 8);
        assert_eq!(tokens[0].token_type, KEYWORD);
        // #feature follows "#OPT " in title → col 6, char = 2 + 5 + 6 = 13
        assert_eq!(tokens[1].delta_start, 13 - 8);
        assert_eq!(tokens[1].token_type, PROPERTY);
    }

    #[test]
    fn assignment_marker_emits_parameter_token() {
        // "- [ ] task @alice" → '@' at 0-based char 11 (indent=0, col=6 in title)
        // length = 1 + 5 = 6 ("@alice")
        let items = p("- [ ] task @alice\n");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        let t = &tokens[0];
        assert_eq!(t.delta_line, 0);
        assert_eq!(t.delta_start, 11);
        assert_eq!(t.length, 6); // "@alice"
        assert_eq!(t.token_type, PARAMETER);
        assert_eq!(t.token_modifiers_bitset, 0);
    }

    #[test]
    fn assignment_on_indented_subtask() {
        // "  - [ ] task @bob" → indent=2, '@' at 0-based char 13
        let items = p("\
- [ ] parent
  - [ ] task @bob
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        let t = &tokens[0];
        assert_eq!(t.delta_line, 1);
        assert_eq!(t.delta_start, 13); // 2 + 5 + 6 = 13
        assert_eq!(t.length, 4); // "@bob"
        assert_eq!(t.token_type, PARAMETER);
    }

    #[test]
    fn assignment_and_property_get_distinct_token_types() {
        // "- [ ] task #feature @alice"
        // #feature at title col 6, char = 11; token_type PROPERTY
        // @alice at title col 15, char = 20; token_type PARAMETER
        let items = p("- [ ] task #feature @alice\n");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].delta_start, 11);
        assert_eq!(tokens[0].token_type, PROPERTY);
        assert_eq!(tokens[1].delta_start, 20 - 11); // delta from prev token
        assert_eq!(tokens[1].token_type, PARAMETER);
    }
}
