# Assignment / Completion Validation (E013)

Ensures only assigned people can mark a task as done. This is a gentle
nudge, not access control (see [MANIFESTO.md](../MANIFESTO.md)) — a user can
always bypass it by committing anyway.

## How it works

1. The check requires a git repository and a resolvable identity for the
   current user (see [Identity resolution](#identity-resolution) below). If
   either is unavailable, the check is skipped for that run — `agile check`
   logs a warning on the terminal so this isn't silent; the language server
   (LSP) skips it without a warning, since there's no terminal to print to.
2. For each `*.agile.md` file, the check compares the working-copy content
   against a base git ref (`HEAD` by default) to detect tasks that just
   transitioned to `[x]`.
3. For each such transition, it collects the task's `@user`/`@group`
   markers:
   - No assignment marker at all → anyone may complete it.
   - Otherwise, authorized identities are the directly-assigned users, plus
     every member of any assigned group. The current identity must match at
     least one.
4. A mismatch is reported as `E013` (see [Checks](checks.md)).

## Identity resolution

The acting identity is resolved as:

- `--as <user-key>` if given (see [CI/CD flags](#cicd-flags) below), or
  otherwise
- `git config user.email` / `git config user.name`, matched against each
  `[Users.X]`'s `git_emails` (first) and `git_names` (fallback) in
  `mdagile.toml` (see [Configuration](config.md)).

An identity that doesn't match any `[Users.X]` entry, is considered unauthorized. 

The check is skipped when the identity is **fully undeterminable**: not
inside a git repo, or `git config user.email`/`user.name` are both empty,
and no `--as` override was given. `agile check` warns on the terminal in
both cases; the LSP skips silently either way. The "not a git repo" warning
can be silenced project-wide for projects that intentionally don't use git —
see `[General] warn_when_not_a_git_repo` in [Configuration](config.md).

## CI/CD flags

Running `agile check` in a CI/CD pipeline (e.g. against a pull request) has
two problems the defaults don't solve: the CI runner's own git identity
isn't the PR author's, and the checked-out branch is usually already fully
committed (so there's no working-copy-vs-`HEAD` diff to detect a
transition). Two flags address this:

### `--as <user-key>`

```sh
agile check --as alice
```

Overrides identity resolution entirely with a literal `[Users.X]` key —
`git config` is not consulted at all. There's no email/`git_names` fallback
matching for `--as`: the value must be an exact config key. A value that
doesn't match any configured user is treated as an unrecognized identity
(unauthorized for assigned tasks), consistent with the strictness rule
above — not a distinct CLI usage error.

### `--base <git-ref>`

```sh
agile check --base origin/main
```

Overrides the git ref used as the "old" side of the diff (default `HEAD`) —
e.g. a PR's base branch, a specific SHA, or a ref computed via `git
merge-base`. The "new" side is always the on-disk working directory (there's
no equivalent `--head`-style flag), which covers the common CI case where
the PR's code is already checked out.

An invalid/non-existent `--base` ref is a **hard CLI error** — reported on
stderr with exit status `1` — distinct from an authorization violation. It's
never silently skipped, since a typo'd ref would otherwise make the check
quietly do nothing.

### Combined example

```sh
agile check --as "$PR_AUTHOR" --base "$PR_BASE_SHA"
```

## Scope

`--as` and `--base` are supported only by the `agile check` CLI command.
The language server's live diagnostics (re-run on every `didOpen`/
`didChange`) always use the live git identity and `HEAD` — there's no
override support in the editor-integration path.
