use super::{Config, FormConfig};
use crate::parser::{Marker, PropertyForm};

pub(crate) struct ActiveForm<'a> {
    pub name: &'a str,
    pub config: &'a FormConfig,
    pub properties: Vec<&'a str>,
}

impl Config {
    /// Distinct forms in marker order, base before selected branch. Keep all
    /// contributing property names for label-collision diagnostics.
    pub(crate) fn active_forms<'a>(&'a self, markers: &'a [Marker]) -> Vec<ActiveForm<'a>> {
        let mut active: Vec<ActiveForm<'a>> = Vec::new();
        for marker in markers {
            let Marker::Property(property) = marker else {
                continue;
            };
            let Some(config) = self.properties.get(&property.name) else {
                continue;
            };
            let branch_form = match &property.form {
                PropertyForm::BranchResolved(branch) => config
                    .branches
                    .get(branch)
                    .and_then(|branch| branch.form.as_deref()),
                _ => None,
            };
            for name in config.form.as_deref().into_iter().chain(branch_form) {
                if let Some(existing) = active.iter_mut().find(|form| form.name == name) {
                    if !existing.properties.contains(&property.name.as_str()) {
                        existing.properties.push(&property.name);
                    }
                    continue;
                }
                let Some(form) = self.forms.get(name) else {
                    continue;
                };
                active.push(ActiveForm {
                    name,
                    config: form,
                    properties: vec![&property.name],
                });
            }
        }
        active
    }
}
