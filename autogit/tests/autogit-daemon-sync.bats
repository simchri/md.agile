#!/usr/bin/env bats
# Sync-loop tests for autogit-daemon (see doc/development/autogit.md
# "What it actually does - sync loop"). Each test sets up a bare "remote"
# repo plus a working clone, registers the clone directly in an isolated
# global config (bypassing the `autogit` CLI, since these tests exercise the
# daemon's own config reading), then runs `autogit-daemon --once` and
# inspects the resulting git state. Log output/notification publishing is
# assumed to work and is intentionally not asserted on (see autogit.bats),
# except for the "error logging" tests, which check that a failing external
# command's captured stdout/stderr is recorded in the log.

setup() {
  REPO_ROOT="$(cd "$(dirname "$BATS_TEST_FILENAME")/../.." && pwd)"
  AUTOGIT_DAEMON="$REPO_ROOT/autogit/bin/autogit-daemon"

  export AUTOGIT_LOG_DIR="$BATS_TEST_TMPDIR/autogit-logs"
  export AUTOGIT_STATE_DIR="$BATS_TEST_TMPDIR/autogit-state"
  export AUTOGIT_GLOBAL_CONFIG="$BATS_TEST_TMPDIR/global.toml"

  REMOTE="$BATS_TEST_TMPDIR/remote.git"
  git init -q --bare "$REMOTE"

  REPO="$BATS_TEST_TMPDIR/repo"
  git clone -q "$REMOTE" "$REPO"
  git -C "$REPO" config user.email "test@example.com"
  git -C "$REPO" config user.name "test"

  echo "hello" > "$REPO/tracked.txt"
  git -C "$REPO" add tracked.txt
  git -C "$REPO" commit -q -m "initial commit"

  register_repo
  write_local_config "# autogit local config"
  git -C "$REPO" push -q origin HEAD
}

# Writes the global config directly (isolated by AUTOGIT_GLOBAL_CONFIG),
# registering $REPO with autogit globally enabled.
register_repo() {
  cat > "$AUTOGIT_GLOBAL_CONFIG" <<EOF
enabled = true
repos = [
  "$REPO",
]
EOF
}

# Writes .autogit.toml and commits it (it's a git-tracked file per
# doc/development/autogit.md), so it never itself shows up as a dirty/
# untracked file in the assertions below.
write_local_config() {
  printf '%s\n' "$1" > "$REPO/.autogit.toml"
  git -C "$REPO" add .autogit.toml
  git -C "$REPO" commit -q -m "autogit config"
}

local_remote_heads_match() {
  [ "$(git -C "$REPO" rev-parse HEAD)" = "$(git -C "$REMOTE" rev-parse HEAD)" ]
}

@test "clean repo with no changes: cycle completes, nothing committed or pushed" {
  before="$(git -C "$REPO" rev-parse HEAD)"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [ "$(git -C "$REPO" rev-parse HEAD)" = "$before" ]
  local_remote_heads_match
}

@test "modified tracked file with no validation commands is committed and pushed" {
  echo "changed" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [ -z "$(git -C "$REPO" status --porcelain)" ]
  local_remote_heads_match
  [ "$(git -C "$REPO" log -1 --format=%s)" != "initial commit" ]
}

@test "failing validation command blocks commit" {
  write_local_config "validation_commands = [
  \"false\",
]
validation_timeouts = [
  \"5\",
]"
  echo "changed" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [ -n "$(git -C "$REPO" status --porcelain)" ]
  [ "$(git -C "$REPO" log -1 --format=%s)" = "autogit config" ]
}

@test "untracked file is left alone by default (stage_untracked=false)" {
  echo "new" > "$REPO/untracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [[ "$(git -C "$REPO" status --porcelain)" == *"?? untracked.txt"* ]]
  [ "$(git -C "$REPO" log -1 --format=%s)" = "autogit config" ]
}

@test "untracked file is committed and pushed when stage_untracked=true" {
  write_local_config "stage_untracked = true"
  echo "new" > "$REPO/untracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [ -z "$(git -C "$REPO" status --porcelain)" ]
  local_remote_heads_match
  git -C "$REMOTE" show HEAD:untracked.txt
}

@test "repo in an abnormal git state (detached HEAD) is skipped" {
  git -C "$REPO" checkout -q --detach HEAD
  echo "changed" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [ -n "$(git -C "$REPO" status --porcelain)" ]
}

@test "globally off: no registered repo is synced" {
  cat > "$AUTOGIT_GLOBAL_CONFIG" <<EOF
enabled = false
repos = [
  "$REPO",
]
EOF
  echo "changed" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [ -n "$(git -C "$REPO" status --porcelain)" ]
}

@test "a leftover per-repo 'enabled = false' is ignored: registration alone decides, repo is synced" {
  write_local_config "enabled = false"
  echo "changed" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [ -z "$(git -C "$REPO" status --porcelain)" ]
  local_remote_heads_match
}

@test "diverging non-conflicting commits are reconciled via rebase and then pushed" {
  # A second clone commits+pushes a change to a different file, simulating
  # another machine syncing in between this daemon's polls.
  OTHER="$BATS_TEST_TMPDIR/other-clone"
  git clone -q "$REMOTE" "$OTHER"
  git -C "$OTHER" config user.email "other@example.com"
  git -C "$OTHER" config user.name "other"
  echo "from other machine" > "$OTHER/other.txt"
  git -C "$OTHER" add other.txt
  git -C "$OTHER" commit -q -m "other machine change"
  git -C "$OTHER" push -q origin HEAD

  echo "changed" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [ -z "$(git -C "$REPO" status --porcelain)" ]
  local_remote_heads_match
  [ -f "$REPO/other.txt" ]
  [ "$(git -C "$REPO" log -1 --format=%s)" != "initial commit" ]
}

@test "real content conflicts trigger conflict resolution: backup branch created, repo reset to remote" {
  OTHER="$BATS_TEST_TMPDIR/other-clone"
  git clone -q "$REMOTE" "$OTHER"
  git -C "$OTHER" config user.email "other@example.com"
  git -C "$OTHER" config user.name "other"
  echo "other machine's version" > "$OTHER/tracked.txt"
  git -C "$OTHER" add tracked.txt
  git -C "$OTHER" commit -q -m "other machine conflicting change"
  git -C "$OTHER" push -q origin HEAD

  echo "this machine's conflicting version" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  local_remote_heads_match
  backup_count="$(git -C "$REPO" branch --list 'autogit-backup/*' | wc -l)"
  [ "$backup_count" -eq 1 ]
}

@test "backup branch retention: expired backup branches are pruned, recent ones kept, when configured" {
  write_local_config "\
backup_branch_retention_days = 14
"
  name="$(basename "$REPO")"
  old_ts="$(date -u -d '30 days ago' +%Y%m%dT%H%M%S)"
  recent_ts="$(date -u -d '1 day ago' +%Y%m%dT%H%M%S)"
  git -C "$REPO" branch "autogit-backup/$name/$old_ts" HEAD
  git -C "$REPO" branch "autogit-backup/$name/$recent_ts" HEAD

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  old_listing="$(git -C "$REPO" branch --list "autogit-backup/$name/$old_ts")"
  recent_listing="$(git -C "$REPO" branch --list "autogit-backup/$name/$recent_ts")"
  [ -z "$old_listing" ]
  [ -n "$recent_listing" ]
}

@test "backup branch retention: unset (default) never deletes backup branches" {
  name="$(basename "$REPO")"
  old_ts="$(date -u -d '365 days ago' +%Y%m%dT%H%M%S)"
  git -C "$REPO" branch "autogit-backup/$name/$old_ts" HEAD

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  old_listing="$(git -C "$REPO" branch --list "autogit-backup/$name/$old_ts")"
  [ -n "$old_listing" ]
}

@test "commit message strategy: default message includes extracted keywords only (no diffstat)" {
  cat > "$REPO/tracked.txt" <<'EOF'
fn calculate_widget_total() {
    let widget_count = 42;
    widget_count
}
EOF

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  subject="$(git -C "$REPO" log -1 --format=%s)"
  echo "commit subject: $subject" >&2
  [[ "$subject" == "autogit: "* ]]
  [[ "$subject" == *"widget"* ]]
  [[ "$subject" != *"file changed"* ]]
  [[ "$subject" != *"files changed"* ]]
}

@test "commit message strategy: commit_message_template config is honored" {
  write_local_config "\
commit_message_template = \"[TEST-TEMPLATE] {summary}\"
"
  echo "changed" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  subject="$(git -C "$REPO" log -1 --format=%s)"
  echo "commit subject: $subject" >&2
  [[ "$subject" == "[TEST-TEMPLATE] "* ]]
  [[ "$subject" != *"file changed"* ]]
  [[ "$subject" != *"files changed"* ]]
}

@test "weekly log rotation: log files older than the retention window are deleted, recent ones kept" {
  mkdir -p "$AUTOGIT_LOG_DIR"
  old_log="$AUTOGIT_LOG_DIR/daemon-2020-01-01.log"
  recent_log="$AUTOGIT_LOG_DIR/daemon-2020-01-06.log"
  echo "old entry" > "$old_log"
  echo "recent entry" > "$recent_log"
  touch -d "8 days ago" "$old_log"
  touch -d "2 days ago" "$recent_log"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [ ! -e "$old_log" ]
  [ -e "$recent_log" ]
}

@test "weekly log rotation: unset retention (custom override to empty) never deletes log files" {
  mkdir -p "$AUTOGIT_LOG_DIR"
  very_old_log="$AUTOGIT_LOG_DIR/daemon-2000-01-01.log"
  echo "ancient entry" > "$very_old_log"
  touch -d "365 days ago" "$very_old_log"

  export AUTOGIT_LOG_RETENTION_DAYS=""
  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  [ -e "$very_old_log" ]
}



# --- error logging: external command output is recorded -------------------
# Exception to the "logs are not asserted on" rule above: these tests check
# that the captured stdout/stderr of a failing external command ends up in
# the logged error message (run_logged helper).

@test "error logging: failing validation command's stdout and stderr are recorded in the log" {
  write_local_config "validation_commands = [
  \"echo STDOUT-MARKER-\$((40+2)); echo STDERR-MARKER-\$((40+2)) 1>&2; false\",
]
validation_timeouts = [
  \"5\",
]"
  echo "changed" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  log="$(cat "$AUTOGIT_LOG_DIR"/daemon-*.log)"
  [[ "$log" == *"validation command failed"* ]]
  # markers are computed at runtime so they can't match the (also logged)
  # command text itself
  [[ "$log" == *"STDOUT-MARKER-42"* ]]
  [[ "$log" == *"STDERR-MARKER-42"* ]]
}

@test "error logging: failing git push's stderr is recorded in the log" {
  mkdir -p "$REMOTE/hooks"
  printf '#!/bin/sh\necho PUSH-REJECTED-BY-HOOK-MARKER 1>&2\nexit 1\n' > "$REMOTE/hooks/pre-receive"
  chmod +x "$REMOTE/hooks/pre-receive"
  echo "changed" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  log="$(cat "$AUTOGIT_LOG_DIR"/daemon-*.log)"
  [[ "$log" == *"git push failed"* ]]
  [[ "$log" == *"PUSH-REJECTED-BY-HOOK-MARKER"* ]]
}

@test "error logging: captured output is truncated to the last 50 lines per stream" {
  write_local_config "validation_commands = [
  \"for i in \$(seq 1 100); do echo OUT-LINE-\$i; done; false\",
]
validation_timeouts = [
  \"5\",
]"
  echo "changed" > "$REPO/tracked.txt"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  log="$(cat "$AUTOGIT_LOG_DIR"/daemon-*.log)"
  [[ "$log" == *"OUT-LINE-100"* ]]
  [[ "$log" == *"OUT-LINE-51"* ]]
  [ -z "$(grep 'OUT-LINE-50$' <<< "$log")" ]
  [[ "$log" == *"truncated"* ]]
}
