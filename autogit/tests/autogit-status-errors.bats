#!/usr/bin/env bats
# Per-repo error state: the daemon records the outcome of each repo's last
# sync cycle in a state file (under AUTOGIT_STATE_DIR), and `autogit status`
# reports it (see doc/development/autogit.md "Error State"). Each test sets up
# a bare "remote" plus a working clone registered with autogit, runs
# `autogit-daemon --once`, then inspects `autogit status` output/exit code.

setup() {
  REPO_ROOT="$(cd "$(dirname "$BATS_TEST_FILENAME")/../.." && pwd)"
  AUTOGIT_BIN="$REPO_ROOT/autogit/bin/autogit"
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
  git -C "$REPO" push -q origin HEAD

  bash -c "cd '$REPO' && '$AUTOGIT_BIN' add ." > /dev/null
  git -C "$REPO" add .autogit.toml
  git -C "$REPO" commit -q -m "autogit config"
  git -C "$REPO" push -q origin HEAD
}

reject_pushes() {
  mkdir -p "$REMOTE/hooks"
  printf '#!/bin/sh\necho PUSH-REJECTED-BY-HOOK-MARKER 1>&2\nexit 1\n' > "$REMOTE/hooks/pre-receive"
  chmod +x "$REMOTE/hooks/pre-receive"
}

@test "error state: repo never synced yet is reported as such, status exits 0" {
  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' status"

  [ "$status" -eq 0 ]
  [[ "$output" == *"last sync: not yet synced"* ]]
}

@test "error state: successful cycle is reported as ok, status exits 0" {
  echo "changed" > "$REPO/tracked.txt"

  "$AUTOGIT_DAEMON" --once 2> /dev/null
  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' status"

  [ "$status" -eq 0 ]
  [[ "$output" == *"last sync: ok"* ]]
  [[ "$output" != *"ERROR"* ]]
}

@test "error state: failing push is reported with step and git's stderr, status exits non-zero" {
  reject_pushes
  echo "changed" > "$REPO/tracked.txt"

  "$AUTOGIT_DAEMON" --once 2> /dev/null
  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' status"

  [ "$status" -ne 0 ]
  [[ "$output" == *"$REPO  [ERROR: push]"* ]]
  [[ "$output" == *"last sync: ERROR in step 'push'"* ]]
  [[ "$output" == *"git push failed"* ]]
  [[ "$output" == *"PUSH-REJECTED-BY-HOOK-MARKER"* ]]
}

@test "error state: failing validation is reported as an error, status exits non-zero" {
  printf 'enabled = true\nvalidation_commands = [\n  "false",\n]\n' > "$REPO/.autogit.toml"
  git -C "$REPO" commit -q -am "failing validation config"
  echo "changed" > "$REPO/tracked.txt"

  "$AUTOGIT_DAEMON" --once 2> /dev/null
  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' status"

  [ "$status" -ne 0 ]
  [[ "$output" == *"last sync: ERROR in step 'validate'"* ]]
  [[ "$output" == *"validation command failed"* ]]
}

@test "error state: a subsequent successful cycle clears the error" {
  reject_pushes
  echo "changed" > "$REPO/tracked.txt"
  "$AUTOGIT_DAEMON" --once 2> /dev/null
  rm "$REMOTE/hooks/pre-receive"

  "$AUTOGIT_DAEMON" --once 2> /dev/null
  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' status"

  [ "$status" -eq 0 ]
  [[ "$output" == *"last sync: ok"* ]]
  [[ "$output" != *"PUSH-REJECTED-BY-HOOK-MARKER"* ]]
}

@test "error state: the error-since timestamp is kept across consecutive failing cycles" {
  reject_pushes
  echo "changed" > "$REPO/tracked.txt"
  "$AUTOGIT_DAEMON" --once 2> /dev/null
  state_file="$(ls "$AUTOGIT_STATE_DIR"/*.status)"
  first_since="$(grep -m1 '^since = ' "$state_file")"
  sleep 1

  "$AUTOGIT_DAEMON" --once 2> /dev/null

  [ "$(grep -m1 '^since = ' "$state_file")" = "$first_since" ]
}

@test "error state: resolved merge conflicts are not reported as an error" {
  OTHER="$BATS_TEST_TMPDIR/other-clone"
  git clone -q "$REMOTE" "$OTHER"
  git -C "$OTHER" config user.email "other@example.com"
  git -C "$OTHER" config user.name "other"
  echo "other machine's version" > "$OTHER/tracked.txt"
  git -C "$OTHER" commit -q -am "other machine conflicting change"
  git -C "$OTHER" push -q origin HEAD
  echo "this machine's conflicting version" > "$REPO/tracked.txt"

  "$AUTOGIT_DAEMON" --once 2> /dev/null
  run bash -c "cd '$REPO' && '$AUTOGIT_BIN' status"

  [ "$status" -eq 0 ]
  [[ "$output" == *"last sync: ok"* ]]
}

@test "error state: status run outside any repo still flags erroring repos and exits non-zero" {
  reject_pushes
  echo "changed" > "$REPO/tracked.txt"

  "$AUTOGIT_DAEMON" --once 2> /dev/null
  run bash -c "cd '$BATS_TEST_TMPDIR' && '$AUTOGIT_BIN' status"

  [ "$status" -ne 0 ]
  [[ "$output" == *"$REPO  [ERROR: push]"* ]]
}
