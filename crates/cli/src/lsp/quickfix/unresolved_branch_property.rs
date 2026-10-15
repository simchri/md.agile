use crate::rules::IssueData;
use tower_lsp::lsp_types::*;

/// E021 quickfix builder.
///
/// Offers one action per branch declared for each still-pending property on
/// the flagged node, replacing that property's pending marker (`#name:...`)
/// with the resolved form for that branch (`#name:branch`). Branches are
/// offered in alphabetical order; properties are processed in the order
/// they're listed in the diagnostic's data (same order they appear in the
/// task's markers -- see `branch_properties::check_unresolved`).
pub fn build(diagnostic: &Diagnostic, doc_text: &str, uri: &Url) -> Vec<CodeAction> {
    let Some((_, toml_content)) = super::read_toml(uri) else {
        return vec![];
    };
    build_with_config(diagnostic, doc_text, uri, &toml_content)
}

fn build_with_config(
    diagnostic: &Diagnostic,
    doc_text: &str,
    uri: &Url,
    toml_content: &str,
) -> Vec<CodeAction> {
    let Some(IssueData::UnresolvedBranchProperty { properties }) = super::issue_data(diagnostic)
    else {
        return vec![];
    };

    let Ok(config) = crate::config::Config::from_str(toml_content) else {
        return vec![];
    };

    let line = diagnostic.range.start.line;
    let Some(line_text) = doc_text.lines().nth(line as usize) else {
        return vec![];
    };

    let mut actions = Vec::new();
    for name in &properties {
        let Some(property) = config.properties.get(name) else {
            continue;
        };

        let needle = format!("#{name}:...");
        let Some(byte_start) = line_text.find(&needle) else {
            continue;
        };
        let byte_end = byte_start + needle.len();
        let (Some(start), Some(end)) = (
            super::super::marker::byte_offset_to_utf16(line_text, byte_start),
            super::super::marker::byte_offset_to_utf16(line_text, byte_end),
        ) else {
            continue;
        };

        let mut branches: Vec<&String> = property.branches.keys().collect();
        branches.sort_unstable();
        for branch in branches {
            let new_text = format!("#{name}:{branch}");
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line,
                        character: start,
                    },
                    end: Position {
                        line,
                        character: end,
                    },
                },
                new_text: new_text.clone(),
            };
            actions.push(super::make_quickfix(
                format!("Resolve #{name} to {new_text}"),
                uri,
                diagnostic,
                edit,
            ));
        }
    }

    actions
}

#[cfg(test)]
#[path = "unresolved_branch_property_tests.rs"]
mod tests;
