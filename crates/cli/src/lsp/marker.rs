use crate::parser::{is_in_code_span, is_marker_boundary, is_marker_escape, is_tick_wrapped};
use tower_lsp::lsp_types::Position;

pub(super) struct MarkerToken {
    pub raw: String,
    pub prefix: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Copy)]
pub(super) enum CursorMode {
    OnMarker,
    Completing,
}

pub(super) fn token_at(
    text: &str,
    position: Position,
    sigil: char,
    mode: CursorMode,
) -> Option<MarkerToken> {
    let line = text.lines().nth(position.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let cursor = char_index_at_utf16(&chars, position.character)?;
    let title_start = if matches!(mode, CursorMode::Completing) {
        let start = task_title_start(&chars)?;
        if cursor < start {
            return None;
        }
        start
    } else {
        0
    };

    let marker_start = match mode {
        CursorMode::OnMarker if chars.get(cursor) == Some(&sigil) => cursor,
        _ => {
            let mut found = None;
            for index in (title_start..cursor).rev() {
                let c = chars[index];
                if matches!(mode, CursorMode::Completing) && is_marker_boundary(c)
                    || matches!(mode, CursorMode::OnMarker) && c.is_ascii_whitespace()
                {
                    break;
                }
                if c == sigil {
                    found = Some(index);
                    break;
                }
            }
            found?
        }
    };

    if is_in_code_span(&chars, marker_start)
        || marker_start > 0 && is_marker_escape(chars[marker_start - 1])
    {
        return None;
    }
    let mut end = marker_start + 1;
    while end < chars.len() && !is_marker_boundary(chars[end]) {
        end += 1;
    }
    if is_tick_wrapped(
        marker_start.checked_sub(1).map(|index| chars[index]),
        chars.get(end).copied(),
    ) {
        return None;
    }
    if matches!(mode, CursorMode::Completing) && chars.get(cursor) == Some(&sigil) {
        return None;
    }
    if matches!(mode, CursorMode::OnMarker) && (cursor >= end || end == marker_start + 1) {
        return None;
    }

    Some(MarkerToken {
        raw: chars[marker_start + 1..end].iter().collect(),
        prefix: chars[marker_start + 1..cursor.max(marker_start + 1).min(end)]
            .iter()
            .collect(),
        start: utf16_offset(&chars, marker_start),
        end: utf16_offset(&chars, end),
    })
}

pub(super) fn char_index_at_utf16(chars: &[char], position: u32) -> Option<usize> {
    let mut offset = 0;
    for (index, c) in chars.iter().enumerate() {
        if offset == position {
            return Some(index);
        }
        offset += c.len_utf16() as u32;
    }
    (offset == position).then_some(chars.len())
}

fn utf16_offset(chars: &[char], end: usize) -> u32 {
    chars[..end].iter().map(|c| c.len_utf16() as u32).sum()
}

pub(super) fn byte_offset_to_utf16(line: &str, byte_offset: usize) -> Option<u32> {
    Some(line.get(..byte_offset)?.encode_utf16().count() as u32)
}

fn task_title_start(chars: &[char]) -> Option<usize> {
    let indent = chars.iter().take_while(|c| **c == ' ').count();
    let prefix = &chars[indent..];
    if prefix.len() < 6
        || prefix[0] != '-'
        || prefix[1] != ' '
        || prefix[2] != '['
        || prefix[4] != ']'
        || prefix[5] != ' '
        || !matches!(prefix[3], ' ' | 'x' | 'X' | '-')
    {
        return None;
    }
    Some(indent + 6)
}
