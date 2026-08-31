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
