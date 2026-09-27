use crate::config::{Config, PropertyConfig};
use crate::lsp::goto_definition::special_marker_at_position;
use crate::parser::{FileItem, Marker, SpecialMarkerKind, Subtask, TASK_LINE_PREFIX_LEN, Task};
use std::path::PathBuf;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

pub(super) fn special_marker_hover(text: &str, line: u32, character: u32) -> Option<Hover> {
    let kind = special_marker_at_position(text, line, character)?;
    let line_text = text.lines().nth(line as usize)?;
    let hover_text = match kind {
        SpecialMarkerKind::Opt => {
            let items = crate::parser::parse(text, PathBuf::from("hover.agile.md"));
            if !task_contains_marker_at(&items, line, character, SpecialMarkerKind::Opt) {
                return None;
            }
            "**#OPT**\n\nOptional subtask. This subtask does not block completion of its parent task."
        }
        SpecialMarkerKind::Milestone => {
            let marker_start = line_text.find("#MILESTONE")?;
            if !line_text.trim().starts_with("#MILESTONE")
                || !character_in_marker(character, marker_start, "#MILESTONE")
            {
                return None;
            }
            let items = crate::parser::parse(text, PathBuf::from("hover.agile.md"));
            if !items.iter().any(|item| {
                matches!(item, FileItem::Milestone(milestone) if milestone.location.line == line as usize + 1)
            }) {
                return None;
            }
            "**#MILESTONE**\n\nA milestone separates tasks in the backlog. It is reached when all tasks before it are complete."
        }
        SpecialMarkerKind::MdAgile => {
            let leading_whitespace = line_text
                .chars()
                .take_while(|character| character.is_whitespace())
                .count();
            if !line_text.trim_start().starts_with("#MDAGILE")
                || !character_in_marker(character, leading_whitespace, "#MDAGILE")
            {
                return None;
            }
            "**#MDAGILE**\n\nReserved for file-level Mdagile directives. The `file.mandatory_property` directive is not currently implemented."
        }
    };
    Some(markdown_hover(hover_text.to_string()))
}

pub(super) fn property_hover(name: &str, property: &PropertyConfig) -> Option<Hover> {
    Some(Hover {
        contents: HoverContents::Markup(property_documentation(name, property)?),
        range: None,
    })
}

pub(super) fn property_documentation(
    name: &str,
    property: &PropertyConfig,
) -> Option<MarkupContent> {
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

    Some(MarkupContent {
        kind: MarkupKind::Markdown,
        value: sections.join("\n\n"),
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

    Some(markdown_hover(sections.join("\n\n")))
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

fn markdown_hover(value: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: None,
    }
}

fn character_in_marker(character: u32, marker_start: usize, marker: &str) -> bool {
    let character = character as usize;
    (marker_start..marker_start + marker.len()).contains(&character)
}

fn task_contains_marker_at(
    items: &[FileItem],
    line: u32,
    character: u32,
    kind: SpecialMarkerKind,
) -> bool {
    items.iter().any(|item| match item {
        FileItem::Task(task) => task_contains_marker(task, line, character, &kind),
        FileItem::Milestone(_) => false,
    })
}

fn task_contains_marker(task: &Task, line: u32, character: u32, kind: &SpecialMarkerKind) -> bool {
    let marker_found = task.location.line == line as usize + 1
        && task.markers.iter().any(|marker| match marker {
            Marker::Special(special) if special.kind == *kind => {
                let start = task.indent + TASK_LINE_PREFIX_LEN + special.column - 1;
                character_in_marker(character, start, &format!("#{}", special.as_str()))
            }
            _ => false,
        });
    marker_found
        || task
            .children
            .iter()
            .any(|child| subtask_contains_marker(child, line, character, kind))
}

fn subtask_contains_marker(
    task: &Subtask,
    line: u32,
    character: u32,
    kind: &SpecialMarkerKind,
) -> bool {
    let marker_found = task.location.line == line as usize + 1
        && task.markers.iter().any(|marker| match marker {
            Marker::Special(special) if special.kind == *kind => {
                let start = task.indent + TASK_LINE_PREFIX_LEN + special.column - 1;
                character_in_marker(character, start, &format!("#{}", special.as_str()))
            }
            _ => false,
        });
    marker_found
        || task
            .children
            .iter()
            .any(|child| subtask_contains_marker(child, line, character, kind))
}
