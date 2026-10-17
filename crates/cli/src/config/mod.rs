use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The two file names mdagile recognizes as its config file, in the order
/// they're preferred when only one is present. Exactly one of these is
/// expected per project directory; both present at once is a
/// [`ConfigError::ConflictingConfig`].
pub const CONFIG_FILE_NAMES: [&str; 2] = ["mdagile.toml", ".mdagile.toml"];

/// Returns the config file path in `dir`, if exactly one of
/// [`CONFIG_FILE_NAMES`] exists there. Doesn't distinguish "neither exists"
/// from "both exist" — callers that need to raise [`ConfigError::ConflictingConfig`]
/// should go through [`Config::load`] instead, which needs both candidate
/// paths anyway to build that error.
pub fn find_config_file_in(dir: &Path) -> Option<PathBuf> {
    CONFIG_FILE_NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}

/// Walks up from `start_dir` (inclusive) through its ancestors, returning the
/// config file path in the nearest directory that has one.
pub fn find_config_file_upwards(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = Some(start_dir);
    while let Some(d) = dir {
        if let Some(path) = find_config_file_in(d) {
            return Some(path);
        }
        dir = d.parent();
    }
    None
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub properties: HashMap<String, PropertyConfig>,
    pub users: HashMap<String, UserConfig>,
    pub groups: HashMap<String, GroupConfig>,
    pub fields: HashMap<String, FieldConfig>,
    pub forms: HashMap<String, FormConfig>,
    pub general: GeneralConfig,
}

/// Project-wide settings that don't belong to any single property/user/group.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralConfig {
    /// Whether `agile check` logs a terminal warning when the E013
    /// assignment/completion check is skipped because the project isn't
    /// inside a git repo at all. Defaults to `true`; set to `false` in
    /// projects that intentionally don't use git, to avoid a warning on
    /// every run.
    pub warn_when_not_a_git_repo: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        GeneralConfig {
            warn_when_not_a_git_repo: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyConfig {
    pub name: String,
    pub brief: Option<String>,
    pub description: Option<String>,
    pub subtasks: Vec<String>,
    /// Parallel array to `subtasks`: if `subtasks_allow_cancel[i]` is `true`, the
    /// required subtask at `subtasks[i]` may be satisfied by cancelling it instead
    /// of completing it. Empty when not configured (no required subtask may be
    /// cancelled).
    pub subtasks_allow_cancel: Vec<bool>,
    /// Sibling task titles (see README.vision.md "Neighbor Tasks") that must be
    /// present at the same level as the task/subtask carrying this property.
    /// Empty when not configured.
    pub neighbortasks: Vec<String>,
    /// Outcome-specific requirements for a Branch Property (see
    /// README.vision.md "Branch Properties"), keyed by branch/outcome name
    /// (e.g. `"passed"`, `"failed"`). Declared as `[Properties.X.branch_name]`
    /// sub-tables in `mdagile.toml`. Empty when this property declares no
    /// branches at all (an ordinary, non-branching property).
    pub branches: HashMap<String, BranchConfig>,
    /// The `[Form.X]` this property's marker renders/validates against (see
    /// README.vision.md "Forms & Fields"), or `None` for an ordinary property
    /// with no form attached. Resolution always goes through this key on the
    /// property itself -- there is no separate `#form_name` marker path.
    pub form: Option<String>,
}

/// Outcome-specific requirements for one branch/outcome of a Branch Property
/// (see README.vision.md "Branch Properties"). These are *additional* to the
/// owning `[Properties.X]`'s own `subtasks`/`neighbortasks` — both apply once
/// the property marker is resolved to this branch (e.g. `#review:passed`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BranchConfig {
    pub subtasks: Vec<String>,
    /// Parallel array to `subtasks`, same semantics as
    /// [`PropertyConfig::subtasks_allow_cancel`].
    pub subtasks_allow_cancel: Vec<bool>,
    pub neighbortasks: Vec<String>,
}

/// A `[Field.X]` entry (see README.vision.md "Forms & Fields"): one named,
/// typed input collected by a `[Form.X]`. Attached to properties indirectly,
/// via a `[Form.X]`'s `fields` list.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldConfig {
    pub name: String,
    /// Display label shown next to the field's value. Falls back to `name`
    /// when not configured.
    pub label: String,
    /// Value substituted for the unfilled placeholder (`___`) when the field
    /// has one. `None` means the plain `___` placeholder is used as-is.
    pub default: Option<String>,
    /// Whether the field's resolved placeholder (`default` above, or the
    /// generic `___` when it has none) counts as a legitimate, deliberately
    /// chosen "filled out" value rather than "left unfilled" (see
    /// [`crate::rules::field_left_unfilled`], E026). Defaults to `false` --
    /// a field still at its placeholder is flagged as unfilled unless a
    /// project explicitly opts in. This only reclassifies the placeholder
    /// itself: the field is still flagged if its value is entirely blank,
    /// regardless of this setting.
    pub default_is_valid_entry: bool,
    /// A regex the field's value must match. Mutually exclusive with
    /// `values` -- at most one of the two may be set.
    pub validation_regex: Option<String>,
    /// A closed set of allowed values. Mutually exclusive with
    /// `validation_regex` -- at most one of the two may be set.
    pub values: Option<Vec<String>>,
    pub multiline: bool,
    /// A configurable hint shown alongside [`crate::rules::field_left_unfilled`]'s
    /// (E026) error message in the LSP, explaining *what* to fill in or
    /// *how* -- e.g. "Use the person's legal name". Falls back to a
    /// generic "Fill in the field's value" when not configured.
    pub help: Option<String>,
}

/// A `[Form.X]` entry (see README.vision.md "Forms & Fields"): a named,
/// ordered group of `[Field.X]` entries, attached to a property via that
/// property's `form` key.
#[derive(Debug, Clone, PartialEq)]
pub struct FormConfig {
    pub name: String,
    /// `[Field.X]` names making up this form's body, in declaration order.
    pub fields: Vec<String>,
    /// Path to an external script that performs whole-form validation (see
    /// README.vision.md "Forms & Fields"). Only stored here for now -- the
    /// trust/execution machinery is a later task.
    pub validation: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserConfig {
    pub name: String,
    /// Email addresses that identify this user's git commits. Used to match
    /// against `git config user.email` for completion-authorization checks.
    pub git_emails: Vec<String>,
    /// Alternate git display names (`git config user.name`), used as a fallback
    /// identity match when no email match is found.
    pub git_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupConfig {
    pub name: String,
    pub brief: Option<String>,
    pub description: Option<String>,
    /// `[Users.X]` keys that belong to this group.
    pub members: Vec<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    ConflictingConfig {
        paths: [std::path::PathBuf; 2],
    },
    /// A `[Properties.X]` entry has a `subtasks_allow_cancel` array whose length
    /// doesn't match `subtasks`.
    PropertyValidation {
        property: String,
        message: String,
    },
    /// A `[Groups.X]` entry's `members` list references a name that isn't
    /// defined as a `[Users.X]` entry.
    GroupValidation {
        group: String,
        message: String,
    },
    /// A `[Field.X]` entry has conflicting or invalid validation settings:
    /// both `validation_regex` and `values` set, or `validation_regex`
    /// doesn't compile as a regex.
    FieldValidation {
        field: String,
        message: String,
    },
    /// A `[Form.X]` entry's `fields` list references a name that isn't
    /// defined as a `[Field.X]` entry, or lists the same field name more
    /// than once.
    FormValidation {
        form: String,
        message: String,
    },
    /// The same `git_emails`/`git_names` value is listed on more than one
    /// `[Users.X]` entry. Left unchecked, [`crate::git::resolve_identity_user`]
    /// would pick one of the matching users based on `HashMap` iteration
    /// order, which is randomized per process and therefore non-deterministic
    /// across runs.
    DuplicateIdentity {
        field: &'static str,
        value: String,
        users: Vec<String>,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "could not read config file: {e}"),
            ConfigError::Parse(e) => write!(f, "invalid config: {e}"),
            ConfigError::ConflictingConfig { paths } => write!(
                f,
                "conflicting config files '{}' and '{}': don't know which config to use",
                paths[0].display(),
                paths[1].display(),
            ),
            ConfigError::PropertyValidation { property, message } => {
                write!(f, "invalid config for property '{property}': {message}")
            }
            ConfigError::GroupValidation { group, message } => {
                write!(f, "invalid config for group '{group}': {message}")
            }
            ConfigError::FieldValidation { field, message } => {
                write!(f, "invalid config for field '{field}': {message}")
            }
            ConfigError::FormValidation { form, message } => {
                write!(f, "invalid config for form '{form}': {message}")
            }
            ConfigError::DuplicateIdentity {
                field,
                value,
                users,
            } => write!(
                f,
                "'{value}' appears in {field} of multiple users: {} (each identity must map to exactly one user)",
                users.join(", "),
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::Parse(e)
    }
}

// Can't `#[serde(deny_unknown_fields)]` alongside `#[serde(flatten)]` (serde
// doesn't support combining the two): any key that isn't one of the explicit
// fields below is captured by `branches` instead, and must itself deserialize
// as a `RawBranchConfig` sub-table (see README.vision.md "Branch
// Properties") -- e.g. a genuine typo'd field holding a scalar/array value
// still surfaces as a `Parse` error, just with a less specific message.
#[derive(serde::Deserialize, Default)]
struct RawPropertyConfig {
    #[serde(default)]
    brief: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    subtasks: Vec<String>,
    #[serde(default)]
    subtasks_allow_cancel: Vec<bool>,
    #[serde(default)]
    neighbortasks: Vec<String>,
    #[serde(default)]
    form: Option<String>,
    #[serde(flatten)]
    branches: HashMap<String, RawBranchConfig>,
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawBranchConfig {
    #[serde(default)]
    subtasks: Vec<String>,
    #[serde(default)]
    subtasks_allow_cancel: Vec<bool>,
    #[serde(default)]
    neighbortasks: Vec<String>,
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawFieldConfig {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    default_is_valid_entry: bool,
    #[serde(default)]
    validation_regex: Option<String>,
    #[serde(default)]
    values: Option<Vec<String>>,
    #[serde(default)]
    multiline: bool,
    #[serde(default)]
    help: Option<String>,
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawFormConfig {
    #[serde(default)]
    fields: Vec<String>,
    #[serde(default)]
    validation: Option<String>,
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawUserConfig {
    #[serde(default)]
    git_emails: Vec<String>,
    #[serde(default)]
    git_names: Vec<String>,
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawGroupConfig {
    #[serde(default)]
    brief: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    members: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGeneralConfig {
    #[serde(default = "default_true")]
    warn_when_not_a_git_repo: bool,
}

impl Default for RawGeneralConfig {
    fn default() -> Self {
        RawGeneralConfig {
            warn_when_not_a_git_repo: true,
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(rename = "Properties", default)]
    properties: HashMap<String, RawPropertyConfig>,
    #[serde(rename = "Users", default)]
    users: HashMap<String, RawUserConfig>,
    #[serde(rename = "Groups", default)]
    groups: HashMap<String, RawGroupConfig>,
    #[serde(rename = "Field", default)]
    fields: HashMap<String, RawFieldConfig>,
    #[serde(rename = "Form", default)]
    forms: HashMap<String, RawFormConfig>,
    #[serde(rename = "General", default)]
    general: RawGeneralConfig,
}

/// Finds a `git_emails`/`git_names` value that's listed on more than one
/// user, using `values` to pick which field to check. Returns the offending
/// value plus the (sorted) keys of every user that lists it.
///
/// Deterministic regardless of `HashMap` iteration order: candidate
/// duplicates are collected first, then sorted by value before picking the
/// first one to report, so the same misconfiguration always produces the
/// same error message.
fn find_duplicate_identity(
    users: &HashMap<String, UserConfig>,
    values: impl Fn(&UserConfig) -> &[String],
) -> Option<(String, Vec<String>)> {
    let mut owners_by_value: HashMap<&str, std::collections::HashSet<&str>> = HashMap::new();
    for (key, user) in users {
        for v in values(user) {
            owners_by_value
                .entry(v.as_str())
                .or_default()
                .insert(key.as_str());
        }
    }

    let mut duplicates: Vec<(&str, Vec<&str>)> = owners_by_value
        .into_iter()
        .filter(|(_, owners)| owners.len() > 1)
        .map(|(value, owners)| {
            let mut owners: Vec<&str> = owners.into_iter().collect();
            owners.sort();
            (value, owners)
        })
        .collect();
    duplicates.sort_by(|a, b| a.0.cmp(b.0));

    duplicates.into_iter().next().map(|(value, owners)| {
        (
            value.to_string(),
            owners.into_iter().map(String::from).collect(),
        )
    })
}

impl Config {
    pub fn from_str(s: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(s)?;

        let fields: HashMap<String, FieldConfig> = raw
            .fields
            .into_iter()
            .map(|(name, raw_field)| {
                if raw_field.validation_regex.is_some() && raw_field.values.is_some() {
                    return Err(ConfigError::FieldValidation {
                        field: name,
                        message:
                            "validation_regex and values are mutually exclusive; set at most one"
                                .to_string(),
                    });
                }
                if let Some(pattern) = &raw_field.validation_regex {
                    if let Err(e) = regex::Regex::new(pattern) {
                        return Err(ConfigError::FieldValidation {
                            field: name,
                            message: format!(
                                "validation_regex '{pattern}' is not a valid regex: {e}"
                            ),
                        });
                    }
                }
                if !raw_field.multiline
                    && raw_field
                        .default
                        .as_deref()
                        .is_some_and(|d| d.contains('\n'))
                {
                    return Err(ConfigError::FieldValidation {
                        field: name,
                        message: "default contains a newline, but multiline is not set to true"
                            .to_string(),
                    });
                }
                let label = raw_field.label.unwrap_or_else(|| name.clone());
                Ok((
                    name.clone(),
                    FieldConfig {
                        name,
                        label,
                        default: raw_field.default,
                        default_is_valid_entry: raw_field.default_is_valid_entry,
                        validation_regex: raw_field.validation_regex,
                        values: raw_field.values,
                        multiline: raw_field.multiline,
                        help: raw_field.help,
                    },
                ))
            })
            .collect::<Result<_, ConfigError>>()?;

        let forms: HashMap<String, FormConfig> = raw
            .forms
            .into_iter()
            .map(|(name, raw_form)| {
                if let Some(unknown) = raw_form
                    .fields
                    .iter()
                    .find(|field| !fields.contains_key(field.as_str()))
                {
                    return Err(ConfigError::FormValidation {
                        form: name,
                        message: format!(
                            "field '{unknown}' is not a defined '[Field.{unknown}]' entry"
                        ),
                    });
                }
                let mut seen = std::collections::HashSet::new();
                if let Some(duplicate) = raw_form
                    .fields
                    .iter()
                    .find(|field| !seen.insert(field.as_str()))
                {
                    return Err(ConfigError::FormValidation {
                        form: name,
                        message: format!("field '{duplicate}' is listed more than once"),
                    });
                }
                let mut seen_labels = std::collections::HashSet::new();
                if let Some(duplicate_label) = raw_form
                    .fields
                    .iter()
                    .filter_map(|field| fields.get(field))
                    .find(|field_config| !seen_labels.insert(field_config.label.as_str()))
                    .map(|field_config| field_config.label.clone())
                {
                    return Err(ConfigError::FormValidation {
                        form: name,
                        message: format!(
                            "two fields share the label '{duplicate_label}' -- field labels must be unique within a form"
                        ),
                    });
                }
                Ok((
                    name.clone(),
                    FormConfig {
                        name,
                        fields: raw_form.fields,
                        validation: raw_form.validation,
                    },
                ))
            })
            .collect::<Result<_, ConfigError>>()?;

        let properties = raw
            .properties
            .into_iter()
            .map(|(name, raw_prop)| {
                if !raw_prop.subtasks_allow_cancel.is_empty()
                    && raw_prop.subtasks_allow_cancel.len() != raw_prop.subtasks.len()
                {
                    return Err(ConfigError::PropertyValidation {
                        property: name.clone(),
                        message: format!(
                            "subtasks_allow_cancel has {} entries but subtasks has {}; they must match in length",
                            raw_prop.subtasks_allow_cancel.len(),
                            raw_prop.subtasks.len(),
                        ),
                    });
                }
                if let Some(form) = &raw_prop.form {
                    if !forms.contains_key(form.as_str()) {
                        return Err(ConfigError::PropertyValidation {
                            property: name.clone(),
                            message: format!(
                                "form '{form}' is not a defined '[Form.{form}]' entry"
                            ),
                        });
                    }
                }
                let branches = raw_prop
                    .branches
                    .into_iter()
                    .map(|(branch, raw_branch)| {
                        if !raw_branch.subtasks_allow_cancel.is_empty()
                            && raw_branch.subtasks_allow_cancel.len() != raw_branch.subtasks.len()
                        {
                            return Err(ConfigError::PropertyValidation {
                                property: name.clone(),
                                message: format!(
                                    "branch '{branch}': subtasks_allow_cancel has {} entries but subtasks has {}; they must match in length",
                                    raw_branch.subtasks_allow_cancel.len(),
                                    raw_branch.subtasks.len(),
                                ),
                            });
                        }
                        Ok((
                            branch,
                            BranchConfig {
                                subtasks: raw_branch.subtasks,
                                subtasks_allow_cancel: raw_branch.subtasks_allow_cancel,
                                neighbortasks: raw_branch.neighbortasks,
                            },
                        ))
                    })
                    .collect::<Result<_, ConfigError>>()?;
                Ok((
                    name.clone(),
                    PropertyConfig {
                        name,
                        brief: raw_prop.brief,
                        description: raw_prop.description,
                        subtasks: raw_prop.subtasks,
                        subtasks_allow_cancel: raw_prop.subtasks_allow_cancel,
                        neighbortasks: raw_prop.neighbortasks,
                        branches,
                        form: raw_prop.form,
                    },
                ))
            })
            .collect::<Result<_, ConfigError>>()?;
        let users: HashMap<String, UserConfig> = raw
            .users
            .into_iter()
            .map(|(name, raw_user)| {
                (
                    name.clone(),
                    UserConfig {
                        name,
                        git_emails: raw_user.git_emails,
                        git_names: raw_user.git_names,
                    },
                )
            })
            .collect();

        if let Some((value, owners)) = find_duplicate_identity(&users, |u| &u.git_emails) {
            return Err(ConfigError::DuplicateIdentity {
                field: "git_emails",
                value,
                users: owners,
            });
        }
        if let Some((value, owners)) = find_duplicate_identity(&users, |u| &u.git_names) {
            return Err(ConfigError::DuplicateIdentity {
                field: "git_names",
                value,
                users: owners,
            });
        }

        let groups = raw
            .groups
            .into_iter()
            .map(|(name, raw_group)| {
                if let Some(unknown) = raw_group
                    .members
                    .iter()
                    .find(|member| !users.contains_key(member.as_str()))
                {
                    return Err(ConfigError::GroupValidation {
                        group: name.clone(),
                        message: format!(
                            "member '{unknown}' is not a defined '[Users.{unknown}]' entry"
                        ),
                    });
                }
                Ok((
                    name.clone(),
                    GroupConfig {
                        name,
                        brief: raw_group.brief,
                        description: raw_group.description,
                        members: raw_group.members,
                    },
                ))
            })
            .collect::<Result<_, ConfigError>>()?;
        Ok(Config {
            properties,
            users,
            groups,
            fields,
            forms,
            general: GeneralConfig {
                warn_when_not_a_git_repo: raw.general.warn_when_not_a_git_repo,
            },
        })
    }

    pub fn load(root: &Path) -> Result<Self, ConfigError> {
        let plain = root.join(CONFIG_FILE_NAMES[0]);
        let dot = root.join(CONFIG_FILE_NAMES[1]);
        match (plain.exists(), dot.exists()) {
            (true, true) => Err(ConfigError::ConflictingConfig {
                paths: [plain, dot],
            }),
            (true, false) => {
                let content = std::fs::read_to_string(&plain)?;
                Config::from_str(&content)
            }
            (false, true) => {
                let content = std::fs::read_to_string(&dot)?;
                Config::from_str(&content)
            }
            (false, false) => Ok(Config::default()),
        }
    }
}

#[cfg(test)]
mod tests;
