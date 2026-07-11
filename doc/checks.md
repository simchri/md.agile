# Checks

`agile check` parses every `*.agile.md` file under the project and reports each
issue as `<path>:<line>: <message>` on stdout. It exits with status `1` if any
issue is found, `0` if the project is clean — use it as a pre-commit hook or a
CI/CD pipeline step.

The same checks run live in the language server (`agilels`) as you edit, with
quickfixes offered where possible.

## Formatting checks

| Code | Name | Description |
|------|------|-------------|
| E001 | Orphaned indented task | A task has leading whitespace but no parent task on the preceding line (usually due to a blank line separating parent and child). Indicates the task should be un-indented to top-level. |
| E002 | Wrong indentation | A task's indentation doesn't match a valid nesting depth. |
| E003 | Wrong body indentation | A task body line is incorrectly indented. |
| E004 | Incomplete parent | A parent task is marked done but has incomplete children. |
| E005 | Missing space after box | A task line is missing a space after the status box (e.g. `- [ ]task` instead of `- [ ] task`). |
| E006 | Invalid box style | The status box contains an unrecognised character (e.g. `- [o] task`, `- [] task`). Valid boxes are `[ ]`, `[x]`, and `[-]`. |
| E007 | Uppercase X | The status box uses an uppercase X (e.g. `- [X] task`). Use lowercase `[x]` instead. |

E001, E002, E003, E005, and E006 all have LSP quickfixes.

## Property / marker checks

These validate `#property` and `@user`/`@group` markers against
`mdagile.toml` (see [Configuration](config.md) and the marker syntax in
[README.vision.md](../README.vision.md)).

| Code | Name | Description |
|------|------|-------------|
| E008 | Undefined property | A `#marker` is used on a task but isn't declared under `[Properties.X]` in `mdagile.toml`. Has an LSP quickfix: corrects the spelling if a close match exists, otherwise offers to add the property to `mdagile.toml`. |
| E009 | Undefined assignment | An `@marker` is used on a task but doesn't match any `[Users.X]` or `[Groups.X]` entry in `mdagile.toml`. Has an LSP quickfix: corrects the spelling if a close match exists, otherwise offers to add the name as a user or group in `mdagile.toml`. |
| E010 | Missing required subtasks | A task has a property (e.g. `#feature`) but is missing one or more of that property's required subtasks. Has an LSP quickfix to insert the missing subtasks. |
| E011 | Unrequired quoted subtask | A subtask uses the quoted syntax (`- [ ] "some subtask"`) — reserved for required subtasks — but isn't declared as required by any of the task's properties. |
| E012 | Cancelled required subtask not allowed | A required subtask was cancelled (`[-]`), but the owning property doesn't allow that subtask to be cancelled (see `subtasks_allow_cancel` in [Configuration](config.md)). |

## Assignment / completion validation

| Code | Name | Description |
|------|------|-------------|
| E013 | Unauthorized completion | A task was marked done by someone who isn't an assignee, nor a member of an assigned group. |

E013 is different from the other checks: it needs git history (it compares
the working copy against a base git ref) and the current user's identity, and
supports CI/CD-specific overrides. See [Assignment / Completion
Validation](assignment-validation.md) for the full details.
