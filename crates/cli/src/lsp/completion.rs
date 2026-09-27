use crate::config::Config;
use crate::lsp::hover::{assignment_documentation, property_documentation};
use crate::lsp::marker::{CursorMode, token_at};
use crate::parser::MARKER_TRAILING_PUNCT;
use std::collections::BTreeSet;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, CompletionTextEdit, Documentation,
    Position, Range, TextEdit,
};

pub(super) fn property_completions(
    text: &str,
    position: Position,
    config: &Config,
) -> Option<CompletionResponse> {
    let (prefix, start, end) = marker_prefix_at_position(text, position, '#')?;
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

pub(super) fn assignment_completions(
    text: &str,
    position: Position,
    config: &Config,
) -> Option<CompletionResponse> {
    let (prefix, start, end) = marker_prefix_at_position(text, position, '@')?;
    let names: BTreeSet<&String> = config
        .users
        .keys()
        .chain(config.groups.keys())
        .filter(|name| name.starts_with(&prefix))
        .collect();

    Some(CompletionResponse::Array(
        names
            .into_iter()
            .map(|name| {
                let kind = if config.users.contains_key(name) {
                    CompletionItemKind::VARIABLE
                } else {
                    CompletionItemKind::MODULE
                };
                CompletionItem {
                    label: format!("@{name}"),
                    kind: Some(kind),
                    documentation: assignment_documentation(name, config)
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
                        new_text: format!("@{name}"),
                    })),
                    ..CompletionItem::default()
                }
            })
            .collect(),
    ))
}

pub(super) fn marker_at_position(text: &str, position: Position) -> Option<char> {
    for sigil in ['#', '@'] {
        if marker_prefix_at_position(text, position, sigil).is_some() {
            return Some(sigil);
        }
    }
    None
}

fn marker_prefix_at_position(
    text: &str,
    position: Position,
    sigil: char,
) -> Option<(String, u32, u32)> {
    let token = token_at(text, position, sigil, CursorMode::Completing)?;
    let full_token = &token.raw;
    if sigil == '#'
        && (["OPT", "MILESTONE", "MDAGILE"].contains(&full_token.as_str())
            || full_token.starts_with("MDAGILE."))
    {
        return None;
    }

    let prefix = token
        .prefix
        .trim_end_matches(|c: char| MARKER_TRAILING_PUNCT.contains(c));
    Some((prefix.to_string(), token.start, token.end))
}
