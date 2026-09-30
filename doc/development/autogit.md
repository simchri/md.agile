# Auto-Git

Planned feature: automatic synchronization of a git repo (commit, push, pull)
so users don't have to think about it. Referenced from `tasks.agile.md`
("Auto-Git" milestone); this document holds the design plan, the task list
itself stays in `tasks.agile.md`.

With the command
```
autogit add .
```
Enable automatic synchronization of the current repo via git.

## Architecture
- new indpendent package (built for both Debian (`.deb`) and rpm-based (`.rpm`) distros)
- bins:
  - systemd service global for the current user, started on log in
  - script to set configuration options (command `autogit`)
- has a configuration file with observed repositories
  - a new repo to observe is added, by adding the path to the global config file
  - global config file path: `${XDG_CONFIG_HOME:-$HOME/.config}/mdagile/autogit.toml` (per-user, matches the existing mdagile-gui settings.rs convention; overridable via `AUTOGIT_GLOBAL_CONFIG` for testing)
- additional configuration files per repository
  - local config file name: `.autogit.toml`, at the repo root. Committed to git (team-shareable policy: validation commands, timeouts, etc.), separate from `mdagile.toml`'s own lifecycle
- config format is toml
  - parsed/written with a hand-rolled minimal bash subset (line-based `key = value`, plus a canonical multi-line array-of-strings form for the global repos list) rather than a real TOML library or an external CLI tool — keeps with the "bash, no build required" architecture goal. Only autogit's own canonical output is guaranteed parseable; arbitrary hand-written TOML is not fully supported
- auto-git actions are typically based on sleep cycles ("polling"); default poll interval is short (e.g. once per minute) — this is load-bearing for the Conflict Resolution strategy below, not just a performance choice
- any relevant config file is re-read before autogit does any action
- technology: Either script is just "bash" for max compatibility. No build required, only packaging. Bats for unit testing
- consider autogit a largely independent project (but keep it in this repository and package it alongside mdagile, for convenience and good integration)

## Other command line actions

set an option value for the current repo:
```
autogit set <option> <value>
```
Print status of autogit (can be called anywhere). Lists all relevant config info and whether the systemd is running or not
Output has to sections
First section: global status
- show global on/off status
- show warning if problem with systemd service
- show list of observed repos
Second section: local status (this repo)
- only if current dir is a registered repo
- repo is currently on/off
- show warning if the current repo local setting is "on" but does not appear in the global config list. User can fix this with `autogit add .`
```
autogit status
```
Turn autogit off for the current repo (remove entry from global config file. Do not delete the local config. Show info message similar to "auto-git off for current repo. Config file .. retained. If you permanently want to un-manage this repo, you can delete this file now."
```
autogit remove .
```
Turn autogit on/off globally:
```
autogit on/off
```

## Global configurations
- list of observed repositories
- globally on / off

## Local Configurations

- on / off state (is auto-git currently turned on for this repo)
- validation commands
  list of commands and expected return codes (optional, default 0), executed in order before any commit is performed. (Commit is only done once all pass). Each command has its own configurable timeout (default: 120 seconds); a command that doesn't complete in that time is killed and treated as a failure for that cycle
- stage untracked files (default: off) — if enabled, autogit stages new/untracked files too (`git add .`/`-A`) instead of the default tracked-files-only behavior (`git add -u`, see Safety Guards)
- validation failure notification threshold (default: 5 minutes) — only surface a user-visible notification once validation commands have been failing continuously for at least this long; failures shorter than this are logged only (not notified), to avoid noise from brief/transient failures. Configurable per repo
- backup branch retention (in days, optional, default: unset/never) — if set, automatically delete `autogit-backup/*` branches (see Conflict Resolution) older than this many days. If unset, backup branches are never automatically deleted
- commit message template (`commit_message_template`, optional, default: unset) — overrides the default `autogit: {summary}` commit message format (see Commit Message Strategy). Supports the placeholders `{summary}` (extracted keywords only, option E; deliberately excludes diffstat counts, which `git log --stat`/`git show` already provide), `{timestamp}`, and `{hostname}`, e.g. `"[{hostname} {timestamp}] {summary}"`

- **Discarded idea: independent "auto commit on/off" + "auto pull push on/off" toggles.** Considered allowing a partial-automation mode where the user commits manually but autogit still handles fetch/reconcile/push. Discarded: the Conflict Resolution strategy's "aggressively discard local work onto a backup branch" tradeoff is only acceptable because the discarded commits are autogit's own frequently-generated, cheap-to-replace commits (see "accepted tradeoff" below, mitigated by short polling intervals). In manual-commit mode, the commits being reconciled/potentially discarded would be user-authored — possibly representing significant hand-written work accumulated across many manual commits between polls — so the same "~1 minute at risk" mitigation no longer holds, and silently resetting user commits onto a backup branch is not acceptable. This would also fragment the sync loop (which step runs depends on which toggle is on) and the Commit Message Strategy (which assumes autogit authors every commit) for comparatively little benefit. Kept as a single combined on/off per repo instead (see "on / off state" above)

## logging
- log to /tmp/autogit/
- time stamped log files, one per day. Rotate every week: log files older than `AUTOGIT_LOG_RETENTION_DAYS` (env var, default 7) are deleted once per sync cycle; set to an empty string to disable rotation
- use log helpers (c.f. snippets.bash)

## What it actually does - sync loop
proposed per-repo cycle order, run each poll interval:
1. re-read local + global config
2. skip this repo for this cycle if: globally off, locally off, or repo is in an "abnormal" git state (see Safety Guards below) — log/surface via `autogit status`, take no further action
3. `git fetch` (read-only, always safe; subject to the 5s git-operation timeout, see Safety Guards)
4. if working tree/index has changes to already-tracked files (or, if this repo's "stage untracked files" option is enabled, any changes): run configured validation commands in order (each subject to its own configurable timeout, default 120s, see Local Configurations)
   - if all pass, stage (`git add -u` by default, or `git add .`/`-A` if opted in) and commit locally
   - if any fail: do not commit, log the failure every cycle; only surface a user-visible notification once validation has been failing continuously for at least the "validation failure notification threshold" (default 5 min, see Local Configurations) — avoids notification noise for brief/transient failures
5. reconcile with the fetched remote:
   - if a fast-forward is possible, fast-forward — no conflict handling needed
   - otherwise, attempt `git rebase` of the local (not-yet-pushed) commits onto the fetched remote ref: since these commits have never been pushed/shared, this is safe under the "never force-push" guard and keeps history linear — no merge-commit noise accumulating from every polling cycle across every machine
   - if the rebase itself hits a conflict, abort it and instead attempt a normal `git merge` (three-way merge): if that completes cleanly with no content conflicts, keep the merge result — still a common case for small, non-overlapping changes, and preserves local history
   - only if that merge also reports actual conflicts, abort it and apply the Conflict Resolution strategy below
6. re-run validation commands once more after any pull/rebase/merge, before pushing (same per-command timeouts) — this guards against a "clean" local commit being combined with a broken remote state
   - if validation now fails: do not push, log/notify (same threshold-based notification as step 4), retry next cycle
7. `git push` (never `--force` — see Safety Guards; subject to the 5s git-operation timeout)
8. log outcome, sleep until next poll

## Conflict Resolution
- if a rebase/merge attempt (step 5) reports actual content conflicts, the top-level strategy is:
  - abort the in-progress merge
  - create a timestamped backup branch/ref at the current local `HEAD` (e.g. `autogit-backup/<repo>/<timestamp>`), preserving all local commits reachable from it — this replaces relying on `git stash` for already-committed work, since stash cannot capture a range of commits, only uncommitted working-tree/index changes
  - this backup branch is local-only and is never pushed to the remote
  - hard-reset the current branch to the fetched remote ref
  - if there were also uncommitted working-tree changes at the time of the reset (on top of the now-backed-up commits), those are captured with `git stash` as before, with an identifiable message (repo path + timestamp) so `git stash list` remains usable even if the user has their own unrelated stash entries
  - inform the user about the situation, including how to inspect/recover the backup branch and any stash entry
    - system notification
    - logging
- this is intentional and applies even to already-committed local (autogit) commits, not just uncommitted working-tree changes: on a genuine (non-clean) merge conflict, local history is aggressively set aside (onto the backup branch) rather than merged/rebased through
- accepted tradeoff: this is deliberately somewhat silent/lossy in the rare case where the backup branch (or stash) is never recovered. The mitigation is the short poll interval (see Architecture) — with an active network connection, at most one poll interval's worth of work (e.g. ~1 minute) is ever at risk of being set-aside-and-forgotten. For that residual edge case, the backup branch/stash is considered sufficient recovery
- backup branch retention: by default, backup branches are never automatically deleted (they're local-only, so this is not team-visible clutter, just local disk/branch-list growth). A repo can opt in to automatic cleanup via the "backup branch retention" local config option (in days); when set, any branch matching the `autogit-backup/<repo>/*` naming convention older than the configured number of days is deleted automatically each cycle. This relies fully on the naming convention (no other bookkeeping) to identify which branches are eligible. Deletion is logged but never surfaced as a user notification

## Safety Guards
- never run `git push --force` (or any equivalent history-rewriting push), under any circumstance
- always pull/reconcile with the remote before pushing, never the reverse
- never add new/untracked files to version control by default: autogit only stages and commits changes (modifications/deletions) to files already tracked by git (`git add -u`, not `git add .`/`git add -A`). This can be overridden per-repo via the "stage untracked files" local config option; adding new files otherwise remains a deliberate, manual user action
- before doing anything else in a cycle, detect and skip repos in an "abnormal" git state, warning instead of acting:
  - detached HEAD
  - mid-rebase / mid-merge / mid-cherry-pick / mid-bisect
  - unborn branch (no commits yet)
  - dirty/uninitialized submodules
- these guards apply even if the repo is otherwise configured "on"; treat them as a hard stop for that cycle, not a one-off failure to retry blindly
- accepted, out of scope: autogit acting concurrently with a user's own manual git usage (e.g. mid-way through staging/crafting a commit) may conflict or produce unexpected results. This is not specially guarded against; the mitigation is that the user can turn autogit off for the repo (`autogit remove .` / local on-off) whenever they want to do manual git work
- autogit only ever operates on the current/checked-out branch (whatever it is at the time of a cycle) and never switches, creates, or manages branches itself; branch management (creating, switching, tracking upstream) is entirely the user's responsibility before/while autogit is on for a repo
- every git network operation (`git fetch`, `git push`) is subject to a fixed 5 second timeout; if it doesn't complete within that time, it is killed, treated as a failure for that cycle (logged), and retried next poll — this bounds how long a hanging network op can stall a repo's cycle
- every configured validation command has its own timeout, configurable per command (default: 120 seconds, see Local Configurations) — deliberately separate from and much longer than the git-operation timeout, since validation commands (e.g. running a test suite) routinely take far longer than a network call. On timeout, the command is killed and treated as a failure for that cycle (logged; still follows the notification threshold above)
- a repo's cycle failing unexpectedly (e.g. an unforeseen git error) never aborts the daemon process or other registered repos' cycles that poll: each repo's cycle is isolated, logged as an error, and simply skipped for that cycle, with the daemon continuing on to the next registered repo

## Commit Message Strategy (draft options)
options considered (or offer as a config setting):
- **A. Fixed generic message**: e.g. `autogit: sync <timestamp>` — simplest, but produces a meaningless, repetitive history
- **B. File list summary**: e.g. `autogit: update foo.rs, bar.md (+2 more)` — more informative, needs truncation for large changesets
- **C. Diffstat summary**: e.g. `autogit: 3 files changed (+42/-7)` — compact, consistent length, no filename noise
- **D. Templated, including machine identity**: combine timestamp + hostname (useful for multi-machine setups when debugging who/what synced) + one of the above, configurable via local config (e.g. `commit_message_template`)
- **E. Keyword extraction (retained option)**: naive bash-only term-frequency heuristic over the changed lines of the diff, no external dependencies (fits "bash, max compatibility"):
  ```bash
  STOPWORDS="the fn let pub use def import return if else for while class impl struct enum mod pub(crate) const static mut self"

  git diff --cached -U0 -- . \
    | grep -E '^[+-][^+-]' \
    | grep -oE '[A-Za-z_][A-Za-z0-9_]{2,}' \
    | grep -vixwFf <(tr ' ' '\n' <<< "$STOPWORDS") \
    | sort | uniq -c | sort -rn | head -5
  ```
  - `-U0` + `^[+-]` isolates changed lines only (not context)
  - `grep -oE` tokenizes identifiers/words
  - the stopword list is embedded directly in the script (a `STOPWORDS` variable), not a separate file — no extra file to ship/read/keep in sync; filters common language noise, kept language-agnostic since repos vary
  - top N most-frequent surviving tokens become the "keywords" that make up the commit message summary (e.g. `autogit: parser, rule, checker`) — deliberately *not* combined with a diffstat (C): counts of files/insertions/deletions are already normal `git log --stat`/`git show` information, redundant to restate in the subject line
  - caveats: crude (no stemming/no real relevance weighting, biased toward long, frequently-repeated identifiers); a real TF-IDF/NLP approach would need extra dependencies (e.g. Python), against the bash-only architecture goal
  - **this is the retained option** for the initial implementation
