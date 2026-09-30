#!/usr/bin/env bats
# Scaffolding tests: only check that the placeholder scripts run and log,
# not any real autogit behavior (there isn't any yet).

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

@test "autogit logs its invocation to the cli log file" {
  run "$AUTOGIT_BIN"
  [ "$status" -eq 0 ]

  log_file="$AUTOGIT_LOG_DIR/cli-$(date +%F).log"
  [ -f "$log_file" ]
  grep -q "autogit invoked" "$log_file"
}

@test "autogit-daemon --once logs a heartbeat and exits" {
  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  log_file="$AUTOGIT_LOG_DIR/daemon-$(date +%F).log"
  [ -f "$log_file" ]
  grep -q "heartbeat" "$log_file"
}
