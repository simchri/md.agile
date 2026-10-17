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

/// Finds the form field line at `line` (0-based) and the `[Field.X]` config
/// its label resolves to via the forms of the owning task's properties. If
/// several of the task's forms declare the same label (an already-flagged
/// collision), the first matching one wins.
pub(super) fn field_config_at_line(
    text: &str,
    line: u32,
    config: &Config,
) -> Option<(crate::parser::FieldLine, crate::config::FieldConfig)> {
    let items = crate::parser::parse(text, std::path::PathBuf::new());
    let mut result = None;
    crate::rules::for_each_node(&items, |node| {
        if result.is_some() {
            return;
        }
        let Some(field_line) = node
            .fields()
            .iter()
            .find(|f| f.location.line == line as usize + 1)
        else {
            return;
        };
        let field_config = node.markers().iter().find_map(|marker| {
            let crate::parser::Marker::Property(prop) = marker else {
                return None;
            };
            let form = config
                .forms
                .get(config.properties.get(&prop.name)?.form.as_ref()?)?;
            form.fields
                .iter()
                .filter_map(|name| config.fields.get(name))
                .find(|f| f.label == field_line.label)
        });
        if let Some(field_config) = field_config {
            result = Some((field_line.clone(), field_config.clone()));
        }
    });
    result
}

/// Completes the value of a single-line form field (`__Label:__ value`) from
/// its configured closed set of `values`. The edit replaces the whole current
/// value.
pub(super) fn field_value_completions(
    text: &str,
    position: Position,
    config: &Config,
) -> Option<CompletionResponse> {
    let (field_line, field_config) = field_config_at_line(text, position.line, config)?;
    if field_line.multiline || field_config.multiline {
        return None;
    }
    let allowed = field_config.values?;

    let line = text.lines().nth(position.line as usize)?;
    let indent = line.len() - line.trim_start().len();
    let value_start_bytes = indent + "__".len() + field_line.label.len() + ":__ ".len();
    let utf16_len = |s: &str| s.encode_utf16().count() as u32;
    let start = utf16_len(line.get(..value_start_bytes)?);
    if position.character < start {
        return None;
    }
    let end = utf16_len(line);
    let current = &field_line.value;
    let typed = current.trim();
    let range = Range {
        start: Position {
            line: position.line,
            character: start,
        },
        end: Position {
            line: position.line,
            character: end,
        },
    };

    // The placeholder (`___`, or a configured default) isn't a typed prefix.
    let is_placeholder = typed.is_empty()
        || typed == "___"
        || config.fields.values().any(|f| {
            f.label == field_line.label && f.default.as_deref().map(str::trim) == Some(typed)
        });
    let typed_lower = typed.to_lowercase();

    Some(CompletionResponse::Array(
        allowed
            .iter()
            .enumerate()
            .filter(|(_, value)| is_placeholder || value.to_lowercase().starts_with(&typed_lower))
            .map(|(index, value)| CompletionItem {
                label: value.clone(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                sort_text: Some(format!("{index:04}")),
                // Server already filtered; keep clients from re-filtering
                // against the existing value (e.g. the `___` placeholder).
                filter_text: Some(current.clone()),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range,
                    new_text: value.clone(),
                })),
                ..CompletionItem::default()
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

#[cfg(test)]
#[path = "completion_tests.rs"]
mod tests;
