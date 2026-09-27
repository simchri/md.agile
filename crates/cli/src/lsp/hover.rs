use crate::config::PropertyConfig;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

pub(super) fn property_hover(name: &str, property: &PropertyConfig) -> Option<Hover> {
    let mut sections = vec![format!("**#{name}**")];

    if let Some(brief) = property
        .brief
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        sections.push(format!("**Brief:**\n\n{brief}"));
    }
    if let Some(description) = property
        .description
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        sections.push(format!("**Description:**\n\n{description}"));
    }
    if !property.subtasks.is_empty() {
        let subtasks = property
            .subtasks
            .iter()
            .map(|subtask| format!("- {subtask}"))
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!("**Required subtasks:**\n\n{subtasks}"));
    }

    if sections.len() == 1 {
        return None;
    }

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: sections.join("\n\n"),
        }),
        range: None,
    })
}
