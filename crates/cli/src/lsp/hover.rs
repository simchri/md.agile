use crate::config::{Config, PropertyConfig};
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

pub(super) fn property_hover(name: &str, property: &PropertyConfig) -> Option<Hover> {
    let mut sections = vec![format!("**#{name}**")];

    if let Some(brief) = property
        .brief
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        sections.push(brief.to_string());
    }
    if let Some(description) = property
        .description
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        sections.push(description.to_string());
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

pub(super) fn assignment_hover(name: &str, config: &Config) -> Option<Hover> {
    let user = config.users.get(name);
    let group = config.groups.get(name);
    if user.is_none() && group.is_none() {
        return None;
    }

    let mut sections = vec![format!("**@{name}**")];
    match (user, group) {
        (Some(_), Some(group)) => {
            sections.push(format!(
                "This assignment refers to both user `{name}` and group `{name}`."
            ));
            sections.push(group_members_section(&group.members));
        }
        (Some(_), None) => sections.push(format!("Assigned to user `{name}`.")),
        (None, Some(group)) => {
            sections.push(format!("Assigned to group `{name}`."));
            sections.push(group_members_section(&group.members));
        }
        (None, None) => return None,
    }
    sections.push(
        "Assignments determine who is eligible to work on this task and who may mark it complete."
            .to_string(),
    );

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: sections.join("\n\n"),
        }),
        range: None,
    })
}

fn group_members_section(members: &[String]) -> String {
    let members = if members.is_empty() {
        "No members are configured.".to_string()
    } else {
        members
            .iter()
            .map(|member| format!("- `{member}`"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!("**Members:**\n\n{members}")
}
