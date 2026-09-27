use crate::config::{Config, PropertyConfig};
use crate::lsp::goto_definition::special_marker_at_position;
use crate::lsp::marker::char_index_at_utf16;
use crate::parser::{FileItem, Marker, SpecialMarkerKind, Subtask, TASK_LINE_PREFIX_LEN, Task};
use std::path::PathBuf;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

pub(super) fn special_marker_hover(text: &str, line: u32, character: u32) -> Option<Hover> {
    let kind = special_marker_at_position(text, line, character)?;
    let line_text = text.lines().nth(line as usize)?;
    let character = char_index_at_utf16(&line_text.chars().collect::<Vec<_>>(), character)? as u32;
    let hover_text = match kind {
        SpecialMarkerKind::Opt => {
            let items = crate::parser::parse(text, PathBuf::from("hover.agile.md"));
            if !task_contains_marker_at(&items, line, character, SpecialMarkerKind::Opt, line_text)
            {
                return None;
            }
            "**#OPT**\n\nOptional subtask. This subtask does not block completion of its parent task."
        }
        SpecialMarkerKind::Milestone => {
            let marker_start = line_text[..line_text.find("#MILESTONE")?].chars().count();
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

    append_metadata(&mut sections, &property.brief, &property.description);
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
    Some(Hover {
        contents: HoverContents::Markup(assignment_documentation(name, config)?),
        range: None,
    })
}

pub(super) fn assignment_documentation(name: &str, config: &Config) -> Option<MarkupContent> {
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
            append_metadata(&mut sections, &group.brief, &group.description);
            sections.push(group_members_section(&group.members));
        }
        (Some(_), None) => sections.push(format!("Assigned to user `{name}`.")),
        (None, Some(group)) => {
            sections.push(format!("Assigned to group `{name}`."));
            append_metadata(&mut sections, &group.brief, &group.description);
            sections.push(group_members_section(&group.members));
        }
        (None, None) => return None,
    }
    sections.push(
        "Assignments determine who is eligible to work on this task and who may mark it complete."
            .to_string(),
    );

    Some(MarkupContent {
        kind: MarkupKind::Markdown,
        value: sections.join("\n\n"),
    })
}

fn append_metadata(
    sections: &mut Vec<String>,
    brief: &Option<String>,
    description: &Option<String>,
) {
    for text in [brief, description] {
        if let Some(text) = text.as_deref().filter(|text| !text.trim().is_empty()) {
            sections.push(text.to_string());
        }
    }
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
    line_text: &str,
) -> bool {
    items.iter().any(|item| match item {
        FileItem::Task(task) => task_contains_marker(task, line, character, &kind, line_text),
        FileItem::Milestone(_) => false,
    })
}

fn task_contains_marker(
    task: &Task,
    line: u32,
    character: u32,
    kind: &SpecialMarkerKind,
    line_text: &str,
) -> bool {
    let marker_found = task.location.line == line as usize + 1
        && matches_special_marker(&task.markers, task.indent, character, kind, line_text);
    marker_found
        || task
            .children
            .iter()
            .any(|child| subtask_contains_marker(child, line, character, kind, line_text))
}

fn subtask_contains_marker(
    task: &Subtask,
    line: u32,
    character: u32,
    kind: &SpecialMarkerKind,
    line_text: &str,
) -> bool {
    let marker_found = task.location.line == line as usize + 1
        && matches_special_marker(&task.markers, task.indent, character, kind, line_text);
    marker_found
        || task
            .children
            .iter()
            .any(|child| subtask_contains_marker(child, line, character, kind, line_text))
}

fn matches_special_marker(
    markers: &[Marker],
    indent: usize,
    character: u32,
    kind: &SpecialMarkerKind,
    line_text: &str,
) -> bool {
    markers.iter().any(|marker| match marker {
        Marker::Special(special) if special.kind == *kind => {
            let byte_start = indent + TASK_LINE_PREFIX_LEN + special.column - 1;
            line_text.get(..byte_start).is_some_and(|before| {
                character_in_marker(
                    character,
                    before.chars().count(),
                    &format!("#{}", special.as_str()),
                )
            })
        }
        _ => false,
    })
}
