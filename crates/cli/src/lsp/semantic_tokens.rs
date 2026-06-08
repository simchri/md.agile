//! LSP Semantic Tokens for `.agile.md` files.
//!
//! Highlights built-in special markers as `keyword` tokens:
//!
//! - `#OPT` and `#MDAGILE` appear as markers on task/subtask lines.
//! - `#MILESTONE: Name` appears as a standalone header line
//!   (`FileItem::Milestone`) — the `#MILESTONE` keyword is highlighted
//!   at column 0 of that line.

use crate::parser::{FileItem, Marker, SpecialMarker, Subtask};
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenType};

// ── Legend ────────────────────────────────────────────────────────────────────

/// Token type legend to advertise in `initialize`.
/// Index positions correspond to `SemanticToken.token_type` values.
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD, // 0 — #OPT / #MILESTONE / #MDAGILE
];

const KEYWORD: u32 = 0;

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
                    length: "#MILESTONE".len() as u32,
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
}

fn collect_subtasks(subtasks: &[Subtask], raw: &mut Vec<RawToken>) {
    for sub in subtasks {
        collect_markers(&sub.markers, sub.location.line, sub.indent, raw);
        collect_subtasks(&sub.children, raw);
    }
}

/// Collect semantic tokens for all special markers on a single task/subtask line.
///
/// `location_line` is 1-based; `indent` is the number of leading spaces.
/// Each `SpecialMarker` stores the 1-based column of `#` within the title
/// text (after the `"  - [ ] "` prefix). The 0-based character in the full
/// source line is therefore `indent + 5 + column`.
fn collect_markers(
    markers: &[Marker],
    location_line: usize,
    indent: usize,
    raw: &mut Vec<RawToken>,
) {
    let line = (location_line - 1) as u32;
    for marker in markers {
        if let Marker::Special(special) = marker {
            let column = match special {
                SpecialMarker::Opt { column }
                | SpecialMarker::Milestone { column }
                | SpecialMarker::MdAgile { column } => *column,
            };
            let name_len = special.as_str().len();
            // indent spaces + "- [ ] " (6 chars) + 1-based column → 0-based char
            let character = (indent + 5 + column) as u32;
            let length = (1 + name_len) as u32; // '#' + name
            raw.push(RawToken {
                line,
                character,
                length,
            });
        }
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
            token_type: KEYWORD,
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

    // ── #MDAGILE ─────────────────────────────────────────────────────────────

    #[test]
    fn mdagile_marker_on_task_line() {
        // "- [ ] config #MDAGILE\n"
        // title text: "config #MDAGILE"
        // "#MDAGILE" at position 7 → col = 8
        // char = 0 + 5 + 8 = 13
        let items = p("- [ ] config #MDAGILE\n");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].delta_start, 13);
        assert_eq!(tokens[0].length, 8); // "#MDAGILE"
    }

    // ── Multiple markers / delta encoding ────────────────────────────────────

    #[test]
    fn two_markers_on_same_line_have_correct_deltas() {
        // Subtask: "  - [ ] #OPT thing #MDAGILE"
        // title: "#OPT thing #MDAGILE"
        // #OPT at col 1, char = 2+5+1 = 8, len 4
        // #MDAGILE: "#OPT thing #MDAGILE" — #MDAGILE starts at pos 11 → col 12
        // char for #MDAGILE = 2 + 5 + 12 = 19
        let items = p("\
- [ ] parent
  - [ ] #OPT thing #MDAGILE
");
        let tokens = build_tokens(&items);
        assert_eq!(tokens.len(), 2);
        // First token: #OPT
        assert_eq!(tokens[0].delta_line, 1);
        assert_eq!(tokens[0].delta_start, 8);
        assert_eq!(tokens[0].length, 4);
        // Second token: #MDAGILE — same line, delta_start relative to prev
        assert_eq!(tokens[1].delta_line, 0);
        assert_eq!(tokens[1].delta_start, 19 - 8); // 11
        assert_eq!(tokens[1].length, 8);
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
    fn property_markers_do_not_emit_tokens() {
        // #feature is a property, not a special marker
        let items = p("- [ ] task #feature\n");
        assert!(build_tokens(&items).is_empty());
    }
}
