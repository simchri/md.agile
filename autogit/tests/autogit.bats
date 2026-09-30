#!/usr/bin/env bats
# Scaffolding tests: only check that the placeholder scripts run, not any
# real autogit behavior (there isn't any yet). Log output/publishing is
# assumed to work and is intentionally not asserted on here.

setup() {
  REPO_ROOT="$(cd "$(dirname "$BATS_TEST_FILENAME")/../.." && pwd)"
  AUTOGIT_BIN="$REPO_ROOT/autogit/bin/autogit"
  AUTOGIT_DAEMON="$REPO_ROOT/autogit/bin/autogit-daemon"

  # Isolate logging into a throwaway directory so tests never touch the
  # real /tmp/autogit/ or leave files behind.
  export AUTOGIT_LOG_DIR="$BATS_TEST_TMPDIR/autogit-logs"
}

@test "autogit prints hello world" {
  run "$AUTOGIT_BIN"
  [ "$status" -eq 0 ]
  [ "$output" = "hello world" ]
}

@test "autogit-daemon --once exits successfully" {
  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]
}
