use crate::config::{Config, PropertyConfig};
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

    let make_item = |name: &str, property: &PropertyConfig, text: String| CompletionItem {
        label: text.clone(),
        kind: Some(CompletionItemKind::PROPERTY),
        documentation: property_documentation(name, property).map(Documentation::MarkupContent),
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
            new_text: text,
        })),
        ..CompletionItem::default()
    };

    Some(CompletionResponse::Array(
        properties
            .into_iter()
            .flat_map(|(name, property)| {
                // A property that declares branches (see README.vision.md
                // "Branch Properties") must always be written pending
                // (`#name:...`) or resolved (`#name:branch`), never plainly
                // -- see E023 "Plain Branching Property". Suggest the
                // pending form, plus one item per declared branch (sorted
                // alphabetically) so the user can also jump straight to a
                // known outcome, instead of the bare `#name` form that
                // would immediately be flagged by that check.
                if property.branches.is_empty() {
                    vec![make_item(name, property, format!("#{name}"))]
                } else {
                    let mut items = vec![make_item(name, property, format!("#{name}:..."))];
                    let mut branches: Vec<&String> = property.branches.keys().collect();
                    branches.sort_unstable();
                    items.extend(
                        branches
                            .into_iter()
                            .map(|branch| make_item(name, property, format!("#{name}:{branch}"))),
                    );
                    items
                }
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
