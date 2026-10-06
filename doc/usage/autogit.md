# Auto-Git

`autogit` keeps a git repository in sync automatically: it commits your
changes, pulls down what's new, and pushes — on a short timer, in the
background — so you don't have to think about `git add`/`commit`/`pull`/`push`
yourself. It's an independent package, installed and run separately from the
`agile` CLI/LSP, but ships alongside mdagile for convenience.

It is deliberately conservative: it never force-pushes, always reconciles with
the remote before pushing, and never stages new/untracked files.

## Installation

Install the `mdagile-autogit` package alongside (or instead of) the other
mdagile components — see [doc/installation/install.md](../installation/install.md).
Installing the package enables and starts a per-user background service
(`autogit.service`) automatically for the user who ran the install. No other
users are affected; each one opts in for themselves with:

```
systemctl --user enable --now autogit.service
```

## Quick start

Register the current repo:

```
autogit add .
```

That's it — within a minute, the daemon starts fetching, committing local
changes, and pushing/pulling for this repo. Check in on it any time with:

```
autogit status
```

Stop managing a repo (any local configurations made in `.autogit.toml` are retained, but the daemon stops observing this repository):

```
autogit remove .
```

Turn auto-git off/on globally:

```
autogit off
autogit on
```

## Commands

| Command | Effect |
|---|---|
| `autogit add <path>` | Register the repo at `<path>`. Creates its local config file (`.autogit.toml`) if missing. |
| `autogit remove <path>` | Un-register the repo. The local `.autogit.toml` is **not** deleted — delete it yourself if you want to fully un-manage the repo. |
| `autogit on` / `autogit off` | Turn auto-git on/off globally, for every registered repo. |
| `autogit status` | Print global status (service state, list of registered repos, any in error) and, if run inside a git repo, that repo's local status. |

## Checking status

```
$ autogit status
Global status:
  auto-git: on
  service: active (running) since Mon 2026-10-06 09:00:12 CEST; 2h 10min ago
  observed repos:
    - /home/alice/work/project-a
    - /home/alice/work/project-b  [ERROR: push]

Local status (this repo: /home/alice/work/project-a):
  last sync: ok (2026-10-06 11:09:03)
```

If a repo's last cycle failed, `autogit status` shows which step failed, how
long it's been failing, and partial captured error output. The command exits
with a non-zero status if any repo (or the current one) is in an error state,
so you can use it as a health check in scripts.

If the current directory is a git repo that isn't registered, `status` just
says so and tells you how to register it.

## Configuration

### Global config

One file per user, listing which repos are managed and the global on/off
switch:

```
${XDG_CONFIG_HOME:-$HOME/.config}/mdagile/autogit.toml
```

You normally never edit this by hand — use `autogit add`/`remove`/`on`/`off`.

### Local, per-repo config

A `.autogit.toml` file at the root of each managed repo. Unlike the global
config, this file is meant to be committed to git and shared with your team
(it only configures *how* syncing behaves for this repo, not *whether* it's
on — that's always decided by the global config on each machine).

```toml
stage_untracked = false
validation_failure_threshold_seconds = 300
backup_branch_retention_days = 14
commit_message_template = "[{hostname} {timestamp}] {summary}"

validation_commands = [
  "cargo test",
  "cargo fmt --check",
]
validation_timeouts = [
  "300",
  "60",
]
```

| Key | Default | Meaning |
|---|---|---|
| `stage_untracked` | `false` | If `true`, autogit also stages new/untracked files (`git add .`), not just changes to already-tracked files (`git add -u`). |
| `validation_failure_threshold_seconds` | `300` | How long validation must be failing continuously before you get a user-visible notification (brief/transient failures are only logged). |
| `backup_branch_retention_days` | unset (never) | If set, backup branches older than this many days (see [Conflict handling](#conflict-handling)) are automatically deleted. |
| `commit_message_template` | unset (uses the default keyword-based message) | Overrides the commit message format. Placeholders: `{summary}`, `{timestamp}`, `{hostname}`. |
| `validation_commands` / `validation_timeouts` | none | Parallel arrays: commands run (in order) before every commit and again before every push, with a per-command timeout in seconds (default 120s if omitted). All must succeed for the commit/push to proceed. |

There is intentionally no `enabled` key here — registration (`autogit
add`/`remove`) is the only on/off switch for a repo.

## What it actually does, each cycle

Roughly once a minute, for every registered repo:

1. Skip the repo for this cycle if auto-git is off globally, or the repo is in
   an unusual git state (see [Safety guards](#safety-guards)) — nothing is
   touched, and `autogit status` surfaces a warning.
2. `git fetch` from the remote.
3. If there are local changes, run your `validation_commands`; only if they
   all pass does it stage and commit.
4. Reconcile with the remote: fast-forward if possible, otherwise rebase your
   not-yet-pushed commits onto the remote, otherwise merge. If that still
   conflicts, see [Conflict handling](#conflict-handling) below.
5. Run validation again (guards against a clean local commit landing on top of
   a broken remote state).
6. `git push` (never with `--force`).

## Conflict handling

If a real (content) conflict comes up during reconciliation, autogit does not
try to resolve it for you. Instead, it:

- creates a local-only backup branch at your current `HEAD`,
  `autogit-backup/<repo>/<timestamp>`, preserving every local commit;
- stashes any uncommitted working-tree changes, with an identifiable message;
- hard-resets the current branch to the fetched remote;
- sends you a system notification and a log entry explaining what happened and
  how to recover the backup branch / stash.

This is deliberate: on a genuine conflict, autogit sets local history aside
rather than attempt an automatic three-way merge of conflicting content. The
short (roughly one minute) polling interval limits how much work is ever at
risk this way. Recover with normal git commands, e.g.:

```
git log autogit-backup/<repo>/<timestamp>
git cherry-pick ...   # or: git merge / git rebase the backup branch back in
git stash list        # if there was also a stash
```

By default backup branches are kept forever (they're local-only, so this is
just disk/branch-list growth, not team-visible clutter); set
`backup_branch_retention_days` if you'd rather they expire automatically.

## Safety guards

- **Never force-pushes.** Ever.
- **Always fetches/reconciles before pushing**, never the other way round.
- **Never stages untracked files** unless you opt in with `stage_untracked =
  true` — new files always require a deliberate `git add`.
- **Skips repos in an unusual state**: detached `HEAD`, a rebase/merge/
  cherry-pick/bisect already in progress, an unborn branch (no commits yet),
  or dirty/uninitialized submodules. It warns via `autogit status` rather than
  guessing what you want.
- **Never switches branches.** Autogit only ever syncs whatever branch is
  currently checked out; branch management is entirely up to you.
- **Times out network operations** (`fetch`/`push`, 5s) and validation
  commands (configurable, default 120s) rather than hanging indefinitely.
- **Isolates failures per repo** — a failure in one registered repo's cycle
  never stops other repos from being synced, or crashes the daemon.

If you're about to do manual git work in a repo managed by autogit (e.g.
interactively rebasing, or half-crafting a commit), it's simplest to `autogit
remove .` first and `autogit add .` again once you're done.

## Logs

Daemon and CLI logs live per-user under:

```
${XDG_STATE_HOME:-~/.local/state}/autogit/logs/
```

one file per day, kept for a week by default (`AUTOGIT_LOG_RETENTION_DAYS`).
If the systemd service isn't reported as `active (running)` by `autogit
status`, check:

```
journalctl --user -u autogit.service
```

## See also

- [Developer documentation](../development/autogit_implementation.md) — internals, file formats, and extension points.
- [doc/installation/install.md](../installation/install.md) — installing the package.
