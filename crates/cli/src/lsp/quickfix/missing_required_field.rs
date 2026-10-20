use crate::config::{Config, FieldConfig};
use crate::rules::IssueData;
use tower_lsp::lsp_types::*;

/// E024: insert each missing required form field as a new body line after
/// the last existing line of the flagged task/subtask's subtree (same
/// "append after the whole subtree" placement as [`super::missing_subtasks`]
/// and [`super::missing_neighbor_tasks`]).
///
/// A single-line field (`[Field.X]` with `multiline` unset or `false`) is
/// inserted as one `__Label:__ <value>` line, where `<value>` is that
/// field's configured `default` (looked up by label within the task's active
/// forms), falling back to the generic unfilled placeholder `___` when the
/// field has no configured default or its config can't be found at all
/// (e.g. `mdagile.toml` failed to load).
///
/// A multiline field (see [`crate::config::FieldConfig::multiline`]) is
/// instead inserted using the multiline grammar -- consistent with
/// [`crate::rules::multiline_field_filled_inline`] (E028), which rejects a
/// multiline field written inline: a bare `__Label:__` opening line, the
/// (possibly multi-line) default value each on its own indented line below
/// it (or no content line at all when there's no configured default -- a
/// blank multiline field is recognized as unfilled by
/// [`crate::rules::field_left_unfilled`], E026, same as an inline field left
/// at the generic `___` placeholder). No closing `...` marker is added --
/// per the multiline field grammar, the field's content runs until the
/// next task or an explicit closing marker, so it is still correctly
/// terminated by whatever follows.
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    build_one(diagnostic, doc_text, uri).into_iter().collect()
}

fn build_one(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let IssueData::MissingRequiredField { missing } = super::issue_data(diagnostic)? else {
        return None;
    };

    if missing.is_empty() {
        return None;
    }

    // Best-effort: a missing/unparsable config just means every field falls
    // back to the generic placeholder, same as the parser default.
    let config = super::read_toml(uri).and_then(|(_, text)| Config::from_str(&text).ok());

    let node_line = diagnostic.range.start.line;
    let mut active_fields = None;
    if let Some(config) = &config {
        let items = crate::parser::parse(doc_text, Default::default());
        crate::rules::for_each_node(&items, |node| {
            if node.location().line == node_line as usize + 1 {
                let forms = config.active_forms(node.markers());
                if !forms.is_empty() {
                    active_fields = Some(
                        forms
                            .iter()
                            .flat_map(|form| form.config.fields.iter().cloned())
                            .collect::<Vec<_>>(),
                    );
                }
            }
        });
    }
    let lines: Vec<&str> = doc_text.lines().collect();
    let node_line_text = lines.get(node_line as usize)?;
    let node_indent = node_line_text.chars().take_while(|c| *c == ' ').count();
    let field_indent = node_indent + 2;

    let last_line = super::missing_subtasks::find_last_subtree_line(&lines, node_line, node_indent);
    let last_line_text = lines.get(last_line as usize)?;
    let last_line_len = last_line_text.len() as u32;

    let mut seen = std::collections::HashSet::new();
    let insert_text: String = missing
        .iter()
        .filter(|label| seen.insert(label.as_str()))
        .map(|label| {
            let field = config.as_ref().and_then(|config| {
                if let Some(keys) = &active_fields {
                    keys.iter()
                        .filter_map(|key| config.fields.get(key))
                        .find(|field| field.label == *label)
                } else {
                    field_config_for_label(config, label)
                }
            });
            format_field_insertion(field, label, field_indent)
        })
        .collect::<Vec<_>>()
        .join("");

    let edit = TextEdit {
        range: Range {
            start: Position {
                line: last_line,
                character: last_line_len,
            },
            end: Position {
                line: last_line,
                character: last_line_len,
            },
        },
        new_text: insert_text,
    };

    Some(super::make_quickfix(
        "Insert missing required field(s)",
        uri,
        diagnostic,
        edit,
    ))
}

/// Builds the inserted text for one missing field (including its leading
/// `\n`), in either the single-line or multiline grammar depending on
/// `label`'s configured [`FieldConfig::multiline`].
fn format_field_insertion(
    field_config: Option<&FieldConfig>,
    label: &str,
    field_indent: usize,
) -> String {
    let indent = " ".repeat(field_indent);
    let default = field_config.and_then(|f| f.default.clone());

    if field_config.is_some_and(|f| f.multiline) {
        // No closing `...` marker is inserted: per the multiline field
        // grammar, the field's content runs until the next task or an
        // explicit closing marker, so the field is still correctly
        // terminated by whatever follows (next task, or end of file).
        let mut block = format!("\n{indent}__{label}:__");
        if let Some(default) = &default {
            for line in default.lines() {
                block.push_str(&format!("\n{indent}{line}"));
            }
        }
        block
    } else {
        let value = default.unwrap_or_else(|| "___".to_string());
        format!("\n{indent}__{label}:__ {value}")
    }
}

/// Finds the `[Field.X]` whose `label` matches `label` exactly (the same
/// matching key the E024/E026/E028 checks themselves use).
fn field_config_for_label<'a>(config: &'a Config, label: &str) -> Option<&'a FieldConfig> {
    config.fields.values().find(|f| f.label == label)
}

#[cfg(test)]
#[path = "missing_required_field_tests.rs"]
mod tests;
