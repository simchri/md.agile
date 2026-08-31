//! Splits a task/subtask title into plain text and highlighted "order"/
//! "marker" tokens, for GUI display.
//!
//! The parser keeps a subtask's leading order prefix (`"1. "`) and any
//! `#`/`@` markers inline in the raw title text (rather than stripping
//! them out), so they read correctly wherever the title is rendered as
//! plain text (e.g. `agile task next`). The GUI additionally wants those
//! same substrings visually distinguished (an "order" look, and a
//! "marker" pill look) — but must not *also* render them a second time in
//! a separate badge/pill row next to the title, which would just show the
//! same information twice. [`tokenize_title`] locates those substrings
//! within the title text itself (using the `order`/`markers` data the
//! server already computed) and splits the title into a sequence of
//! [`TitleToken`]s the UI can render with different styling per kind,
//! covering the whole title exactly once.

/// One piece of a tokenized title: either ordinary text, or a substring
/// that should be visually highlighted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleToken {
    /// Ordinary text, rendered as-is.
    Plain(String),
    /// The leading order prefix (e.g. `"1."`), found via `order`. Never
    /// includes the separating space after it — that's kept as a
    /// [`TitleToken::Plain`] so callers don't need to add spacing
    /// themselves.
    Order(String),
    /// A `#`/`@` marker (e.g. `"#feature"`, `"@alice"`), found via
    /// `markers`, verbatim as it appears in the title.
    Marker(String),
}

/// Splits `title` into a sequence of [`TitleToken`]s: the leading order
/// prefix implied by `order` (if `title` actually starts with it), each
/// substring in `markers` found (in order) within the remaining text, and
/// everything else as plain text.
///
/// `markers` must be in the same left-to-right order the marker text
/// actually appears in `title` (as returned by the parser) — each marker is
/// searched for only in the text *after* the previous match, so out-of-order
/// input would fail to match correctly. A marker whose text can't be found
/// at all (e.g. some mismatch between the formatted marker and how it
/// actually appears in the title) is simply left out of the highlighted
/// tokens — its text still ends up in the surrounding plain text, so
/// nothing is ever lost, just left unstyled.
pub fn tokenize_title(title: &str, order: Option<u32>, markers: &[String]) -> Vec<TitleToken> {
    let mut tokens = Vec::new();
    let mut rest = title;

    if let Some(n) = order {
        let prefix = format!("{n}. ");
        if let Some(stripped) = rest.strip_prefix(prefix.as_str()) {
            tokens.push(TitleToken::Order(format!("{n}.")));
            push_plain(&mut tokens, " ");
            rest = stripped;
        }
    }

    for marker in markers {
        if marker.is_empty() {
            continue;
        }
        let Some(idx) = rest.find(marker.as_str()) else {
            continue;
        };
        let (before, after) = rest.split_at(idx);
        push_plain(&mut tokens, before);
        tokens.push(TitleToken::Marker(marker.clone()));
        rest = &after[marker.len()..];
    }

    push_plain(&mut tokens, rest);

    tokens
}

/// Returns `true` for the characters that end a marker name — a relaxed,
/// client-side approximation of the parser's own boundary rule (see
/// `mdagile::parser::is_marker_boundary`, which isn't reachable here since
/// this module must also compile for the wasm32 client, where the
/// `mdagile` crate isn't available).
fn is_marker_boundary(c: char) -> bool {
    c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '\'' | '"')
}

/// Trailing punctuation trimmed off the end of a scanned marker name, same
/// set the parser strips (see `mdagile::parser::MARKER_TRAILING_PUNCT`).
const MARKER_TRAILING_PUNCT: [char; 4] = [':', ';', ',', '.'];

/// Splits arbitrary free-form `text` (e.g. a task's body lines) into a
/// sequence of [`TitleToken`]s, self-detecting any `#`/`@` markers within it
/// rather than relying on a pre-computed marker list — unlike
/// [`tokenize_title`], body text has no such list available, since the
/// parser only extracts markers from title lines.
///
/// This is a relaxed approximation of the parser's real marker grammar
/// (whitespace/bracket-delimited names, common trailing punctuation
/// trimmed), without its escape (`\#`) or tick-wrap (`'#literal'`) rules —
/// acceptable here since those are rare in prose body text, and any
/// mismatch just leaves a would-be marker unstyled rather than losing text.
pub fn tokenize_text(text: &str) -> Vec<TitleToken> {
    let mut tokens = Vec::new();
    let mut rest = text;

    loop {
        let Some(start) = rest.find(['#', '@']) else {
            push_plain(&mut tokens, rest);
            break;
        };

        let (before, from_sigil) = rest.split_at(start);
        let name_start = from_sigil.char_indices().nth(1).map(|(i, _)| i);
        let Some(name_start) = name_start else {
            // Sigil is the very last character — nothing follows it, so
            // it can't be a marker name; treat the rest as plain text.
            push_plain(&mut tokens, rest);
            break;
        };

        let name_end = from_sigil[name_start..]
            .find(is_marker_boundary)
            .map(|i| name_start + i)
            .unwrap_or(from_sigil.len());
        let raw_name = &from_sigil[name_start..name_end];
        let trimmed_end = raw_name.trim_end_matches(MARKER_TRAILING_PUNCT.as_slice());

        if trimmed_end.is_empty() {
            // No usable name after the sigil (e.g. a bare "#" or "@" or
            // one immediately followed by punctuation) — not a marker;
            // keep scanning right after the sigil so it isn't matched
            // again in an infinite loop.
            push_plain(&mut tokens, before);
            push_plain(&mut tokens, &from_sigil[..name_start]);
            rest = &from_sigil[name_start..];
            continue;
        }

        let marker_end = name_start + trimmed_end.len();
        push_plain(&mut tokens, before);
        tokens.push(TitleToken::Marker(from_sigil[..marker_end].to_string()));
        rest = &from_sigil[marker_end..];
    }

    tokens
}

/// Appends `text` to `tokens` as plain text, merging into a trailing
/// [`TitleToken::Plain`] if there is one, instead of creating an adjacent
/// duplicate — keeps the token sequence minimal and predictable. Does
/// nothing for empty `text`.
fn push_plain(tokens: &mut Vec<TitleToken>, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(TitleToken::Plain(last)) = tokens.last_mut() {
        last.push_str(text);
    } else {
        tokens.push(TitleToken::Plain(text.to_string()));
    }
}

#[cfg(test)]
#[path = "title_highlight_tests.rs"]
mod tests;
