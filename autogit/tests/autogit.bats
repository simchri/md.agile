#!/usr/bin/env bats
# Scaffolding tests: only check that the placeholder daemon script runs
# (autogit-daemon has no real functionality yet), plus the actual `autogit`
# CLI behavior implemented so far (config writing + status printing). Log
# output/publishing is assumed to work and is intentionally not asserted on,
# except for the "error logging" test, which checks that a failing external
# command's captured stdout/stderr is recorded in the log.

setup() {
  REPO_ROOT="$(cd "$(dirname "$BATS_TEST_FILENAME")/../.." && pwd)"
  AUTOGIT_BIN="$REPO_ROOT/autogit/bin/autogit"
  AUTOGIT_DAEMON="$REPO_ROOT/autogit/bin/autogit-daemon"

  # Isolate logging and the global config into throwaway locations so tests
  # never touch the real ~/.local/state/autogit/ or ~/.config/mdagile/autogit.toml.
  export AUTOGIT_LOG_DIR="$BATS_TEST_TMPDIR/autogit-logs"
  export AUTOGIT_STATE_DIR="$BATS_TEST_TMPDIR/autogit-state"
  export AUTOGIT_GLOBAL_CONFIG="$BATS_TEST_TMPDIR/global.toml"

  # A throwaway git repo to run repo-scoped commands (add/remove/set/status) in.
  REPO="$BATS_TEST_TMPDIR/repo"
  mkdir -p "$REPO"
  git -C "$REPO" init -q
  git -C "$REPO" config user.email "test@example.com"
  git -C "$REPO" config user.name "test"
}

@test "autogit with no args prints usage" {
  run "$AUTOGIT_BIN"
  [ "$status" -eq 0 ]
  [[ "$output" == *"usage: autogit"* ]]
}

@test "autogit --help and -h print usage" {
  run "$AUTOGIT_BIN" --help
  [ "$status" -eq 0 ]
  [[ "$output" == *"usage: autogit"* ]]

  run "$AUTOGIT_BIN" -h
  [ "$status" -eq 0 ]
  [[ "$output" == *"usage: autogit"* ]]
}

@test "autogit-daemon --once exits successfully" {
  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]
}

@test "autogit add . registers the repo in the global config and creates a local config" {
  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' add ."
  [ "$status" -eq 0 ]

  [ -f "$AUTOGIT_GLOBAL_CONFIG" ]
  grep -qF "$REPO" "$AUTOGIT_GLOBAL_CONFIG"
  [ -f "$REPO/.autogit.toml" ]
  [ -z "$(grep '^enabled' "$REPO/.autogit.toml")" ]
}

@test "autogit add . is idempotent (repo listed once)" {
  bash -c "cd '$REPO' && '$AUTOGIT_BIN' add ." >/dev/null
  bash -c "cd '$REPO' && '$AUTOGIT_BIN' add ." >/dev/null

  count="$(grep -cF "$REPO" "$AUTOGIT_GLOBAL_CONFIG")"
  [ "$count" -eq 1 ]
}

@test "autogit remove . unregisters the repo but keeps the local config" {
  bash -c "cd '$REPO' && '$AUTOGIT_BIN' add ." >/dev/null

  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' remove ."
  [ "$status" -eq 0 ]

  ! grep -qF "$REPO" "$AUTOGIT_GLOBAL_CONFIG"
  [ -f "$REPO/.autogit.toml" ]
}

@test "autogit add accepts an absolute path argument (not just '.')" {
  run "$AUTOGIT_BIN" add "$REPO"
  [ "$status" -eq 0 ]

  [ -f "$AUTOGIT_GLOBAL_CONFIG" ]
  grep -qF "$REPO" "$AUTOGIT_GLOBAL_CONFIG"
  [ -f "$REPO/.autogit.toml" ]
}

@test "autogit add accepts a relative path argument" {
  run bash -c "cd '$BATS_TEST_TMPDIR' && '$AUTOGIT_BIN' add repo"
  [ "$status" -eq 0 ]

  grep -qF "$REPO" "$AUTOGIT_GLOBAL_CONFIG"
  [ -f "$REPO/.autogit.toml" ]
}

@test "autogit remove accepts a path argument (not just '.')" {
  "$AUTOGIT_BIN" add "$REPO" >/dev/null

  run "$AUTOGIT_BIN" remove "$REPO"
  [ "$status" -eq 0 ]

  ! grep -qF "$REPO" "$AUTOGIT_GLOBAL_CONFIG"
  [ -f "$REPO/.autogit.toml" ]
}

@test "autogit add with a path that is not a git repository fails with an error" {
  run "$AUTOGIT_BIN" add "$BATS_TEST_TMPDIR"
  [ "$status" -ne 0 ]
  [[ "$output" == *"not a git repository"* ]]
}

@test "autogit on/off toggle the global enabled flag" {
  bash -c "cd '$REPO' && '$AUTOGIT_BIN' off" >/dev/null
  grep -q '^enabled = false' "$AUTOGIT_GLOBAL_CONFIG"

  bash -c "cd '$REPO' && '$AUTOGIT_BIN' on" >/dev/null
  grep -q '^enabled = true' "$AUTOGIT_GLOBAL_CONFIG"
}

@test "autogit set no longer exists (per-repo enabled setting removed; registration alone decides)" {
  bash -c "cd '$REPO' && '$AUTOGIT_BIN' add ." >/dev/null

  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' set enabled false"

  [ "$status" -ne 0 ]
  [[ "$output" == *"unknown command: set"* ]]
}

@test "autogit status prints global and local sections for a registered repo" {
  bash -c "cd '$REPO' && '$AUTOGIT_BIN' add ." >/dev/null

  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' status"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Global status:"* ]]
  [[ "$output" == *"auto-git: on"* ]]
  [[ "$output" == *"$REPO"* ]]
  local_section="${output#*Local status}"
  [[ "$local_section" != *"auto-git:"* ]]
  [[ "$local_section" != *"not managed by autogit"* ]]
  [[ "$local_section" == *"last sync:"* ]]
}

@test "autogit status in an unregistered repo without local config only says it is not managed" {
  expected_local_section="\
Local status (this repo: $REPO):
  not managed by autogit. Run 'autogit add .' to register it."

  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' status"

  [ "$status" -eq 0 ]
  [[ "$output" == *"$expected_local_section" ]]
  [[ "$output" != *"WARNING"* ]]
}

@test "autogit status in a removed repo (local config retained) only says it is not managed" {
  bash -c "cd '$REPO' && '$AUTOGIT_BIN' add ." >/dev/null
  bash -c "cd '$REPO' && '$AUTOGIT_BIN' remove ." >/dev/null
  expected_local_section="\
Local status (this repo: $REPO):
  not managed by autogit. Run 'autogit add .' to register it."

  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' status"

  [ "$status" -eq 0 ]
  [[ "$output" == *"$expected_local_section" ]]
}

@test "autogit status warns when the repo is in an abnormal git state (detached HEAD)" {
  echo "hello" > "$REPO/tracked.txt"
  git -C "$REPO" add tracked.txt
  git -C "$REPO" commit -q -m "initial commit"
  bash -c "cd '$REPO' && '$AUTOGIT_BIN' add ." >/dev/null
  git -C "$REPO" checkout -q --detach

  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' status"
  [ "$status" -eq 0 ]
  [[ "$output" == *"warning: repo is in an abnormal git state (detached HEAD)"* ]]
  # printed once (stdout), not additionally echoed to stderr by the log helper
  [[ "$output" != *"WARNING"* ]]
  grep -q "abnormal git state (detached HEAD)" "$AUTOGIT_LOG_DIR"/cli-*.log
}

@test "default log dir is per-user under XDG_STATE_HOME (not a shared /tmp path)" {
  # A single shared /tmp/autogit/ breaks as soon as a second user (e.g. gdm)
  # creates it first: other users then can't write their logs there.
  unset AUTOGIT_LOG_DIR
  export XDG_STATE_HOME="$BATS_TEST_TMPDIR/xdg-state"

  run "$AUTOGIT_BIN" add "$BATS_TEST_TMPDIR"
  [ "$status" -ne 0 ]
  ls "$XDG_STATE_HOME"/autogit/logs/cli-*.log

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]
  ls "$XDG_STATE_HOME"/autogit/logs/daemon-*.log
}

@test "error logging: autogit add on a non-git path records git's stderr in the log" {
  run "$AUTOGIT_BIN" add "$BATS_TEST_TMPDIR"
  [ "$status" -ne 0 ]

  log="$(cat "$AUTOGIT_LOG_DIR"/cli-*.log)"
  [[ "$log" == *"not a git repository: $BATS_TEST_TMPDIR"* ]]
  [[ "$log" == *"stderr:"* ]]
  [[ "$log" == *"fatal:"* ]]
}
