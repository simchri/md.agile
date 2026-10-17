# Configuration: `mdagile.toml`

Project-level configuration — properties, users, groups, fields, forms and
general settings — lives in a
`mdagile.toml` file at the project root. A hidden `.mdagile.toml` is also
recognized; having both `mdagile.toml` and `.mdagile.toml` in the same root is
a conflict error. If neither file exists, an empty (all-defaults) config is
used.

```toml
[Properties]

[Properties.feature]
brief = "New end-user visible functionality of the product"
description = "Not considered features are bugfixes, refactorings, and other internal changes. Features are usually requested by the product owner or the customer."
subtasks = ["design", "implementation", "tests"]
subtasks_allow_cancel = [false, false, true]

[Users]

[Users.alice]

[Users.bob]
git_names = ["Bob Jones", "bobjones"]
git_emails = ["bob@example.com"]

[Groups]

[Groups.devs]
brief = "Development team"
description = "Contributors responsible for implementing product changes."
members = ["alice", "bob"]
```

## Overview of valid keys

Only the following top-level tables are valid; anything else is a parse error.

| Table | Keys |
|-------|------|
| `[Properties.X]` | `brief`, `description`, `subtasks`, `subtasks_allow_cancel`, `neighbortasks`, `form`, plus `[Properties.X.branch_name]` sub-tables |
| `[Properties.X.branch_name]` | `subtasks`, `subtasks_allow_cancel`, `neighbortasks` |
| `[Users.X]` | `git_emails`, `git_names` |
| `[Groups.X]` | `brief`, `description`, `members` |
| `[Field.X]` | `label`, `default`, `default_is_valid_entry`, `values`, `multiline`, `help` |
| `[Form.X]` | `fields`, `validation` |
| `[General]` | `warn_when_not_a_git_repo` |

## `[Properties.X]`

Declares a `#marker` usable on tasks (see the marker syntax in
[README.vision.md](../../README.vision.md)). Referencing an undeclared `#marker`
on a task is [E008](checks.md).

- `brief` — optional short summary of the property.
- `description` — optional longer explanation of the property. These texts are
  available to editor integrations, such as language-server hover.
- `subtasks` — an ordered list of required subtask titles. A task carrying
  this property must have a matching `- [ ] "title"` (quoted) subtask for
  each entry, or [E010](checks.md) is reported.
- `subtasks_allow_cancel` — optional, parallel array to `subtasks`. If
  `subtasks_allow_cancel[i]` is `true`, the required subtask at
  `subtasks[i]` may be satisfied by cancelling it (`[-]`) instead of
  completing it. Defaults to "not allowed" for any subtask not covered.
  Mismatched array lengths are a config validation error.
- `neighbortasks` — a list of required sibling task titles. A task carrying
  this property must have a matching (quoted or unquoted) sibling task for
  each entry, or [E019](checks.md) is reported. Not supported on top-level
  tasks — see [E020](checks.md).
- `form` — optional name of a `[Form.X]` this property's marker renders and
  validates against. It must refer to a defined `[Form.X]`, otherwise it is a
  config validation error.

### `[Properties.X.branch_name]`

Declares a branch outcome for a branching property (see "Branch Properties"
in [README.vision.md](../../README.vision.md)). Declaring at least one
branch here means `X` must always be used pending (`#X:...`) or resolved
(`#X:branch_name`), never in its plain form (`#X`) — [E023](checks.md)
reports that. The marker `#X:...` is pending until resolved to one of its
declared branches (`#X:branch_name`); [E021](checks.md) reports a task
marked done while still pending, and [E022](checks.md) reports a resolved
branch that isn't declared here.

Each `[Properties.X.branch_name]` sub-table accepts the same `subtasks`,
`subtasks_allow_cancel`, and `neighbortasks` keys as `[Properties.X]` itself.
Once a property resolves to a given branch, that branch's requirements apply
*in addition to* the base property's own requirements.

## `[Users.X]`

Declares a user, referenced by an `@marker` on tasks (`@alice`). Referencing
an undeclared `@marker` is [E009](checks.md).

- `git_emails` — email addresses that identify this user's git commits.
  Matched against `git config user.email` for the [assignment / completion
  validation check (E013)](assignment-validation.md). The same email/name
  must not be listed on more than one user — that's a config validation
  error (identity resolution would otherwise be ambiguous).
- `git_names` — display names (`git config user.name`), used as a fallback
  identity match when no email match is found.

## `[Groups.X]`

Declares a group, referenced by an `@marker` on tasks (`@devs`). Grants
authorization to every listed member for E013 purposes.

- `brief` — optional short summary of the group, shown on assignment hover
  and in completion documentation.
- `description` — optional longer explanation of the group, shown after its
  brief on assignment hover and in completion documentation.
- `members` — a list of `[Users.X]` keys belonging to this group. Every name
  listed must correspond to an actually-defined user — an unknown member
  name is a config validation error.

## `[Field.X]`

Declares a named, typed input, collected by a `[Form.X]` (see "Forms & Fields"
in [README.vision.md](../../README.vision.md)). All keys are optional.

- `label` — display label shown next to the value. Defaults to the field name.
  Labels must be unique within a form.
- `default` — value substituted for the unfilled placeholder (`___`). Without
  it, the plain `___` is used. A value containing a newline requires
  `multiline = true`.
- `default_is_valid_entry` — whether the placeholder (the `default`, or `___`)
  counts as a deliberately filled-out value. Defaults to `false`: a field still
  at its placeholder is reported as unfilled ([E026](checks.md)). A blank value
  is always reported.
- `values` — a closed list of allowed values.
- `multiline` — whether the value may span multiple lines. Defaults to `false`.
- `help` — hint shown with the unfilled-field error in the language server.
  Defaults to a generic "Fill in the field's value".

## `[Form.X]`

Declares a named, ordered group of fields, attached to a property through that
property's `form` key.

- `fields` — ordered list of `[Field.X]` names making up the form. Every name
  must be a defined field and may be listed only once.
- `validation` — project-relative path to an external script for whole-form
  validation. It must stay inside the project and may only run once approved
  with `agile trust` (see README); see
  [form_validation_scripts.md](form_validation_scripts.md) for the interface.
  `agile check` doesn't run scripts yet.

## `[General]`

Project-wide settings that don't belong to any single property/user/group.

- `warn_when_not_a_git_repo` — whether `agile check` logs a terminal warning
  when the [assignment / completion validation check
  (E013)](assignment-validation.md) is skipped because the project isn't
  inside a git repo at all. Defaults to `true`; set to `false` in projects
  that intentionally don't use git, to avoid a warning on every run.

## Validation

`agile check` rejects the config outright — before running any per-task
checks — in these cases:

- **Unknown keys** — any key not recognized by the schema above (typos, e.g.
  `git_emial`) causes a hard parse error.
- **Unknown group members** — a `[Groups.X]` `members` entry that doesn't
  match any `[Users.X]` key.
- **Mismatched `subtasks_allow_cancel` length** — must be empty, or exactly
  as long as `subtasks`. Applies the same way to a branch's own `subtasks`/
  `subtasks_allow_cancel` pair within `[Properties.X.branch_name]`.
- **Invalid fields/forms** — a `[Field.X]` with a multiline `default` but
  without `multiline = true`, a `[Form.X]` listing an unknown or
  duplicate field, a `[Properties.X]` `form` naming an undefined form.
- **Duplicate identity** — the same `git_emails`/`git_names` value listed on
  more than one `[Users.X]` entry.

These are reported on stderr and exit the process with status `1`, distinct
from normal per-task check issues (which are printed to stdout as
`<path>:<line>: <message>`).

The language server surfaces the same errors instead of silently falling
back to an empty config: on an invalid/conflicting config it pops a
`window/showMessage` notification (once per distinct error, not on every
keystroke while the same error persists) and publishes a synthetic
diagnostic on the document being validated, so the editor makes clear that
config-dependent checks (E008–E013) are disabled until `mdagile.toml` is
fixed. It also runs only the config-*independent* rule checks while the
config is broken (not the full rule set against an empty placeholder
config) — otherwise every `#marker`/`@marker` in the document would be
reported as spuriously undefined (E008/E009) on top of the real config
error.
