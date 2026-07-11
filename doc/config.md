# Configuration: `mdagile.toml`

Project-level configuration — properties, users, and groups — lives in a
`mdagile.toml` file at the project root. A hidden `.mdagile.toml` is also
recognized; having both `mdagile.toml` and `.mdagile.toml` in the same root is
a conflict error. If neither file exists, an empty (all-defaults) config is
used.

```toml
[Properties]

[Properties.feature]
subtasks = ["design", "implementation", "tests"]
subtasks_allow_cancel = [false, false, true]

[Users]

[Users.alice]

[Users.bob]
git_names = ["Bob Jones", "bobjones"]
git_emails = ["bob@example.com"]

[Groups]

[Groups.devs]
members = ["alice", "bob"]
```

## `[Properties.X]`

Declares a `#marker` usable on tasks (see the marker syntax in
[README.vision.md](../README.vision.md)). Referencing an undeclared `#marker`
on a task is [E008](checks.md).

- `subtasks` — an ordered list of required subtask titles. A task carrying
  this property must have a matching `- [ ] "title"` (quoted) subtask for
  each entry, or [E010](checks.md) is reported.
- `subtasks_allow_cancel` — optional, parallel array to `subtasks`. If
  `subtasks_allow_cancel[i]` is `true`, the required subtask at
  `subtasks[i]` may be satisfied by cancelling it (`[-]`) instead of
  completing it. Defaults to "not allowed" for any subtask not covered.
  Mismatched array lengths are a config validation error.

## `[Users.X]`

Declares a user, referenced by an `@marker` on tasks (`@alice`). Referencing
an undeclared `@marker` is [E009](checks.md).

- `git_emails` — email addresses that identify this user's git commits.
  Matched against `git config user.email` for the [assignment / completion
  validation check (E013)](assignment-validation.md).
- `git_names` — display names (`git config user.name`), used as a fallback
  identity match when no email match is found.

## `[Groups.X]`

Declares a group, referenced by an `@marker` on tasks (`@devs`). Grants
authorization to every listed member for E013 purposes.

- `members` — a list of `[Users.X]` keys belonging to this group. Every name
  listed must correspond to an actually-defined user — an unknown member
  name is a config validation error.

## Validation

`agile check` (and the language server) reject the config outright — before
running any per-task checks — in these cases:

- **Unknown keys** — any key not recognized by the schema above (typos, e.g.
  `git_emial`) causes a hard parse error.
- **Unknown group members** — a `[Groups.X]` `members` entry that doesn't
  match any `[Users.X]` key.
- **Mismatched `subtasks_allow_cancel` length** — must be empty, or exactly
  as long as `subtasks`.

These are reported on stderr and exit the process with status `1`, distinct
from normal per-task check issues (which are printed to stdout as
`<path>:<line>: <message>`).
