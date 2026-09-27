use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Property,
    User,
    Group,
}

impl Kind {
    fn section(self) -> &'static str {
        match self {
            Self::Property => "Properties",
            Self::User => "Users",
            Self::Group => "Groups",
        }
    }
}

struct Declaration {
    kind: Kind,
    name: String,
    line: u32,
}

pub(super) struct DeclarationIndex {
    entries: Vec<Declaration>,
}

impl DeclarationIndex {
    pub(super) fn parse(text: &str) -> Result<Self, toml_edit::TomlError> {
        let document = toml_edit::ImDocument::parse(text)?;
        let mut entries = Vec::new();
        for kind in [Kind::Property, Kind::User, Kind::Group] {
            let Some(section) = document
                .as_table()
                .get(kind.section())
                .and_then(toml_edit::Item::as_table_like)
            else {
                continue;
            };
            for (name, value) in section.iter() {
                if value.as_table_like().is_none() {
                    continue;
                }
                let Some((key, _)) = section.get_key_value(name) else {
                    continue;
                };
                let Some(span) = key.span() else {
                    log::warn!(
                        "missing source span for {} declaration {name}",
                        kind.section()
                    );
                    continue;
                };
                let line = text[..span.start]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count() as u32;
                entries.push((
                    span.start,
                    Declaration {
                        kind,
                        name: name.to_owned(),
                        line,
                    },
                ));
            }
        }
        entries.sort_by_key(|(start, _)| *start);
        Ok(Self {
            entries: entries.into_iter().map(|(_, entry)| entry).collect(),
        })
    }

    pub(super) fn line(&self, kind: Kind, name: &str) -> Option<u32> {
        self.entries
            .iter()
            .find(|entry| entry.kind == kind && entry.name == name)
            .map(|entry| entry.line)
    }

    pub(super) fn assignment_line(&self, name: &str) -> Option<u32> {
        self.entries
            .iter()
            .find(|entry| matches!(entry.kind, Kind::User | Kind::Group) && entry.name == name)
            .map(|entry| entry.line)
    }

    pub(super) fn names(&self, kinds: &[Kind]) -> Vec<&str> {
        let mut seen = HashSet::new();
        self.entries
            .iter()
            .filter(|entry| kinds.contains(&entry.kind) && seen.insert(entry.name.as_str()))
            .map(|entry| entry.name.as_str())
            .collect()
    }
}

#[cfg(test)]
#[path = "declarations_tests.rs"]
mod tests;
