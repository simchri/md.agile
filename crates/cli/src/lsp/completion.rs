use crate::config::Config;
use crate::lsp::hover::property_documentation;
use crate::parser::{MARKER_TRAILING_PUNCT, is_in_code_span, is_marker_boundary, is_marker_escape};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, CompletionTextEdit, Documentation,
    Position, Range, TextEdit,
};

pub(super) fn property_completions(
    text: &str,
    position: Position,
    config: &Config,
) -> Option<CompletionResponse> {
    let (prefix, start, end) = property_prefix_at_position(text, position)?;
    let mut properties: Vec<_> = config
        .properties
        .iter()
        .filter(|(name, _)| name.starts_with(&prefix))
        .collect();
    properties.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));

    Some(CompletionResponse::Array(
        properties
            .into_iter()
            .map(|(name, property)| CompletionItem {
                label: format!("#{name}"),
                kind: Some(CompletionItemKind::PROPERTY),
                documentation: property_documentation(name, property)
                    .map(Documentation::MarkupContent),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: Range {
                        start: Position {
                            line: position.line,
                            character: start,
                        },
                        end: Position {
                            line: position.line,
                            character: end,
                        },
                    },
                    new_text: format!("#{name}"),
                })),
                ..CompletionItem::default()
            })
            .collect(),
    ))
}

pub(super) fn property_prefix_at_position(
    text: &str,
    position: Position,
) -> Option<(String, u32, u32)> {
    let line = text.lines().nth(position.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let cursor = char_index_at_utf16_position(&chars, position.character)?;

    let title_start = task_title_start(&chars)?;
    if cursor < title_start {
        return None;
    }

    let mut sigil = None;
    for index in (title_start..=cursor.min(chars.len().saturating_sub(1))).rev() {
        if index < cursor && is_marker_boundary(chars[index]) {
            break;
        }
        if chars[index] == '#' {
            sigil = Some(index);
            break;
        }
    }
    let sigil = sigil?;

    if is_in_code_span(&chars, sigil)
        || (sigil > 0 && is_marker_escape(chars[sigil - 1]))
        || cursor < sigil + 1
    {
        return None;
    }

    let before_sigil = sigil.checked_sub(1).and_then(|idx| chars.get(idx)).copied();
    let mut marker_end = sigil + 1;
    while marker_end < chars.len() && !is_marker_boundary(chars[marker_end]) {
        marker_end += 1;
    }
    let after_marker = chars.get(marker_end).copied();
    if before_sigil == Some('\'') && after_marker == Some('\'') {
        return None;
    }

    let full_token: String = chars[sigil + 1..marker_end].iter().collect();
    if ["OPT", "MILESTONE", "MDAGILE"].contains(&full_token.as_str())
        || full_token.starts_with("MDAGILE.")
    {
        return None;
    }

    let prefix_end = cursor.min(marker_end);
    let prefix: String = chars[sigil + 1..prefix_end].iter().collect();
    if prefix.is_empty() {
        return Some((
            prefix,
            utf16_offset(&chars, sigil),
            utf16_offset(&chars, marker_end),
        ));
    }

    let clean_prefix = prefix.trim_end_matches(|c: char| MARKER_TRAILING_PUNCT.contains(c));
    Some((
        clean_prefix.to_string(),
        utf16_offset(&chars, sigil),
        utf16_offset(&chars, marker_end),
    ))
}

fn task_title_start(chars: &[char]) -> Option<usize> {
    let indent = chars.iter().take_while(|c| **c == ' ').count();
    let task_prefix = &chars[indent..];
    if task_prefix.len() < 6
        || task_prefix[0] != '-'
        || task_prefix[1] != ' '
        || task_prefix[2] != '['
        || task_prefix[4] != ']'
        || task_prefix[5] != ' '
        || !matches!(task_prefix[3], ' ' | 'x' | 'X' | '-')
    {
        return None;
    }
    Some(indent + 6)
}

fn char_index_at_utf16_position(chars: &[char], position: u32) -> Option<usize> {
    let mut utf16_index = 0;
    for (char_index, character) in chars.iter().enumerate() {
        if utf16_index == position {
            return Some(char_index);
        }
        utf16_index += character.len_utf16() as u32;
    }
    (utf16_index == position).then_some(chars.len())
}

fn utf16_offset(chars: &[char], char_index: usize) -> u32 {
    chars[..char_index]
        .iter()
        .map(|character| character.len_utf16() as u32)
        .sum()
}
