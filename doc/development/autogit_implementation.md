# Auto-Git: Implementation Notes

This document describes how `autogit` is actually implemented: file layout,
internal conventions, state/config formats, and where to make changes. For
the original design rationale and the full list of considered alternatives,
see [doc/development/autogit.md](autogit.md) — that document predates (and
drove) this implementation and is kept as the historical design record; this
one describes the code as it stands. For end-user usage, see
[doc/usage/autogit.md](../usage/autogit.md).

## Why a separate bash package

Autogit is packaged (`mdagile-autogit`) and developed independently of the
Rust `agile`/`agilels`/GUI components, even though it ships from the same
repository:

- **No build step.** The scripts are plain bash, installed byte-for-byte by
  the package. This matches the "runs everywhere, no toolchain required"
  goal for a background daemon that has to survive on arbitrary user
  machines.
- **Bats for tests**, not `cargo test` — see [Testing](#testing).
- Despite the independence, it's packaged *alongside* mdagile (same repo,
  same `make install`/`make package` flow) purely for convenience — there's
  no technical coupling to the Rust crates.

## File layout

```
autogit/
  bin/
    autogit           # CLI: config read/write, `status`, no sync logic
    autogit-daemon     # the sync loop; run by the systemd user service
  systemd/
    autogit.service    # user-level unit, see Service lifecycle below
  packaging/
    postinst           # enables/starts the service for the installing user
    prerm              # disables the service on package removal
  tests/
    autogit.bats
    autogit-daemon-sync.bats
    autogit-daemon-bugreproduction.bats
    autogit-status-errors.bats
scripts/
  package-autogit-deb.sh   # assembles the .deb (see also package-rpm.sh)
doc/
  development/autogit.md                # design doc (historical)
  development/autogit_implementation.md # this file
  usage/autogit.md                      # user guide
```

`autogit` (the CLI) and `autogit-daemon` are two separate scripts with no
shared library file. This is a deliberate project convention (see
"Duplicated, not shared" below), not an oversight.

## Duplicated, not shared

Several pieces of logic exist in *both* `autogit` and `autogit-daemon`,
near-verbatim:

- the `log_line`/`loga`/`logi`/`logw`/`loge` logging helpers (and
  `run_logged`/`format_captured_stream`)
- `abnormal_state_reason` (the Safety Guards git-state check)
- reading the per-repo error-state file (`repo_status_file`/`repo_status_field`/`repo_status_message`)
- `repo_state_key` (must produce the *same* hash in both scripts — see
  [State files](#state-files))

This is intentional: each script stays a single self-contained file with no
install-path lookup for a shared library, which matters for a bash-only,
no-build package that has to be installed as standalone executables. If you
change one of these pieces (e.g. the abnormal-state checks, or the state
file format), **you must change it in both scripts** and keep them in sync.
The `repo_state_key` derivation in particular must remain identical between
`autogit` and `autogit-daemon`, or `autogit status` will look up the wrong
state file for a repo.

## Config file formats

All config is `key = value` TOML-*like* text, parsed with `grep`/`sed`, not a
real TOML library — see
[doc/development/autogit.md](autogit.md#architecture) for the rationale. Only
the canonical form written by autogit's own `global_write`/arrays is
guaranteed to parse correctly; do not expect a hand-crafted TOML file with
comments interleaved inside an array, alternate quoting, etc. to work.

### Global config

Path: `${AUTOGIT_GLOBAL_CONFIG:-${XDG_CONFIG_HOME:-$HOME/.config}/mdagile/autogit.toml}`

```toml
enabled = true
repos = [
  "/abs/path/one",
  "/abs/path/two",
]
```

Written atomically as a whole file by `global_write()` (in `autogit`); reread
in full by `global_read_enabled()`/`global_read_repos()` in *both* scripts
before every daemon cycle and every CLI status/command invocation (config is
always re-read fresh, never cached across the daemon's polling loop).

Key functions (in `autogit`): `global_config_file`, `global_read_enabled`,
`global_read_repos`, `global_repo_registered`, `global_write`,
`global_add_repo`, `global_remove_repo`, `global_set_enabled`.

### Local, per-repo config

Path: `<repo-root>/.autogit.toml`. Deliberately a separate lifecycle from
`mdagile.toml` (so autogit's own config format changes don't entangle with
the Rust config). There is no `enabled` key — see
`local_config_init_if_missing` in `autogit` for the stub file it creates on
`autogit add`.

Read-side lives in `autogit-daemon` (`local_read_stage_untracked`,
`local_read_validation_threshold`, `local_read_backup_retention_days`,
`local_read_commit_message_template`, `local_read_validation_commands`,
`local_read_validation_timeouts`) — each a one-off `grep`+`sed` against a
single key, falling back to the documented default if the key/file is
absent. `validation_commands`/`validation_timeouts` are parsed as the same
multi-line bracketed-array form as the global `repos` list, and are treated
as *parallel arrays* (same index = same command's timeout) — if you add a
command, keep both arrays in sync length-wise; a missing timeout entry falls
back to the 120s default per-command (see `run_validation`).

## State files

Everything here lives under
`${AUTOGIT_STATE_DIR:-${XDG_STATE_HOME:-~/.local/state}/autogit}`, keyed per
repo by `repo_state_key()` — `sha256sum` of the full repo path, truncated to
16 hex chars. (Earlier revision used a naive `/` → `_` substitution; this was
replaced because distinct paths like `.../foo/bar_baz` and `.../foo_bar/baz`
could collide and leak one repo's state into another's — see
`1_tasks.agile.md`'s autogit section for the bug history.)

| File | Written by | Purpose |
|---|---|---|
| `<key>.status` | `record_repo_ok`/`record_repo_error` in `autogit-daemon` | Last sync outcome: `state`/`step`/`since`/`updated` header lines, then (for errors) a blank line + the full captured error message. Read by `autogit status` via `repo_status_file`/`_field`/`_message` (duplicated reader, see above). Written atomically (temp file + `mv`). |
| `<key>.failure_since` | `record_validation_failure`/`clear_failure_state` | Start of the current continuous validation-failure streak, used to gate the user-visible notification threshold. |
| `<key>.notified` | same | Marks that the threshold-exceeded notification has already been sent, so it isn't repeated every cycle. |

None of these are meant to be edited by hand; treat them as the daemon's
private state. They persist across reboots (hence `XDG_STATE_HOME`, not
`/tmp` or `/run`) — a shared, non-per-user path was tried originally and
caused a real production bug (see [Known pitfalls](#known-pitfalls)).

## The sync loop (`autogit-daemon`)

`sync_all_repos` iterates the registered repos once per poll interval,
calling `sync_one_repo` for each, guarded with `if ! sync_one_repo ...; then
...` rather than letting `set -e` take down the whole process — this is what
gives per-repo failure isolation (one repo's git error never stops the
others or kills the daemon). `prune_old_logs` also runs once per cycle
(log rotation, independent of any repo).

`sync_one_repo` (the ~70-line core function) is a fairly literal
implementation of the per-repo cycle described in
[doc/development/autogit.md](autogit.md#what-it-actually-does---sync-loop):
abnormal-state check → `git_timeout fetch` → `has_changes` + `run_validation`
→ `stage_and_commit` → `reconcile` (fast-forward / rebase / merge /
`conflict_resolution`) → `run_validation` again → `git_timeout push` →
`record_repo_ok`/`record_repo_error`.

Notable helper functions, if you need to change behavior:

- `git_timeout` — wraps a git invocation with a 5s `timeout`; used for
  `fetch`/`push` only (not for local, non-network git operations).
- `has_changes` — runs `git status --porcelain` and, unless
  `stage_untracked` is on, filters out untracked-file lines (`^??`) before
  checking for any remaining output, so only changes to already-tracked
  files count by default.
- `commit_message_summary` / `render_commit_message` — Option E keyword
  extraction (see the design doc's "Commit Message Strategy") plus
  `commit_message_template` placeholder substitution
  (`{summary}`/`{timestamp}`/`{hostname}`).
- `conflict_resolution` — backup branch + stash + hard reset + `notify`, see
  the design doc's "Conflict Resolution" section; `prune_backup_branches` is
  the separate, best-effort cleanup step (naming-convention based, no other
  bookkeeping).
- `notify` — best-effort desktop notification; must never fail the cycle
  (wrap in a way that swallows errors if the notification backend is
  unavailable, e.g. no `notify-send`/no display).

## Service lifecycle

`autogit.service` is a **user** systemd unit (`WantedBy=default.target`,
installed to `/usr/lib/systemd/user/`), not a system service — it has to run
as the user whose repos it's syncing (ssh keys, git credential helpers,
etc.), and must not run for every system account.

`packaging/postinst` (shared between the `.deb` `postinst` and the `.rpm`
`%post`, via rpm's `%include` — see `scripts/package-autogit-deb.sh` and
`scripts/package-rpm.sh`) is therefore careful to only enable/start the
service for the *installing* user (`$SUDO_USER`/`logname`), with a fallback
to manually creating the per-user `default.target.wants` symlink if that
user has no live systemd user session (e.g. a non-interactive/CI install).
It deliberately does **not** ship a global
`/usr/lib/systemd/user/default.target.wants/` symlink, which would otherwise
start the daemon for every user on the box, including system accounts like
`gdm` (see [Known pitfalls](#known-pitfalls)). `packaging/prerm` is the
inverse: disables the service on removal.

## Testing

Bats (`autogit/tests/*.bats`), run as part of `make test` (`cargo test &&
bats autogit/tests` — there is no separate autogit-only Make target; see the
Makefile's `test` target). Four files, by concern:

- `autogit.bats` — the `autogit` CLI: config read/write, `add`/`remove`/
  `on`/`off`.
- `autogit-status-errors.bats` — `autogit status` output/exit-code
  behavior against recorded error states.
- `autogit-daemon-sync.bats` — the sync loop itself: fetch/commit/
  reconcile/push, conflict resolution, validation gating.
- `autogit-daemon-bugreproduction.bats` — regression tests for specific
  fixed bugs (e.g. the `repo_state_key` collision, the `autogit add .`
  literal-dot-argument bug) — see `1_tasks.agile.md`'s autogit section for
  the bug history behind these.

When adding a new config option or sync-loop step, add/extend a `.bats` test
first (project-wide TDD convention also applies here), then run just the
relevant file inside the dev container, e.g.:

```
devenv . -a -c "bats autogit/tests/autogit-daemon-sync.bats"
```

## Known pitfalls

- **Don't share paths across users.** A shared, non-per-user path (e.g.
  something under `/tmp/`) for logs or state breaks as soon as another user
  (notably `gdm`) creates it first — this caused a real crash-loop in
  production (service kept restarting with "Permission denied"). Always key
  log/state directories off `XDG_STATE_HOME`/similar, which is per-user by
  construction.
- **Don't ship a global service-enable symlink.** It silently starts the
  daemon for every system account, not just real users. Enable per-user, at
  install time, only for the installing user.
- **Keep `repo_state_key` identical between `autogit` and `autogit-daemon`.**
  A divergence there means `autogit status` reads/writes a different file
  than the daemon, silently showing stale or wrong state.
- **Don't let a single repo's failure take down the loop.** `sync_all_repos`
  must keep guarding each `sync_one_repo` call explicitly; a bare call under
  `set -e` would abort the whole daemon process on the first git error in
  any one registered repo.
- **Validation and git-operation timeouts are intentionally different**
  (120s default vs. 5s) — don't "simplify" these to share one timeout value;
  validation commands (e.g. full test suites) routinely run far longer than
  a network call should ever be allowed to hang.
