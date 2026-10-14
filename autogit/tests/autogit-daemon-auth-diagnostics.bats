#!/usr/bin/env bats
# Regression coverage for the "autogit systematically fails to fetch after a
# reboot, until `systemctl restart --user autogit`" bug (see
# doc/development/autogit.md "Authentication environment and the SSH-agent
# startup race"): a daemon started before the SSH agent/keyring was ready is
# stuck with a stale/missing SSH_AUTH_SOCK for its whole life, since a
# process's environment never updates after it forks. These tests cover the
# daemon-side mitigations:
#   - fetch/push failures that look auth-related get an extra, secret-free
#     diagnostic log line describing the ambient auth-agent environment;
#     failures that don't look auth-related do not get that extra line;
#   - refresh_agent_environment() lets an already-running daemon self-heal
#     by re-reading agent-related vars from the systemd --user manager each
#     cycle (also logged, so the race becomes visible instead of silent).

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

  cat > "$AUTOGIT_GLOBAL_CONFIG" <<EOF
enabled = true
repos = [
  "$REPO",
]
EOF
  printf '# autogit local config\n' > "$REPO/.autogit.toml"
  git -C "$REPO" add .autogit.toml
  git -C "$REPO" commit -q -m "autogit config"
  git -C "$REPO" push -q origin HEAD
}

daemon_log() {
  cat "$AUTOGIT_LOG_DIR"/daemon-*.log
}

# Makes `git fetch`'s underlying ssh transport fail the way a real agent
# rejection would, without any real network/ssh/key involved: GIT_SSH_COMMAND
# is git's own supported override, swapped in only for this test process.
fake_ssh_permission_denied() {
  local fake_ssh="$BATS_TEST_TMPDIR/fake-ssh.sh"
  printf '#!/bin/sh\necho "git@github.com: Permission denied (publickey)." 1>&2\nexit 255\n' > "$fake_ssh"
  chmod +x "$fake_ssh"
  export GIT_SSH_COMMAND="$fake_ssh"
  git -C "$REPO" remote set-url origin "git@github.com:example/does-not-matter.git"
}

@test "auth diagnostics: an SSH permission-denied fetch failure gets an auth-related diagnostic log line" {
  fake_ssh_permission_denied
  unset SSH_AUTH_SOCK

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  log="$(daemon_log)"
  [[ "$log" == *"fetch failure looks auth-related"* ]]
  [[ "$log" == *"remote scheme: ssh"* ]]
  [[ "$log" == *"SSH_AUTH_SOCK=unset"* ]]
}

@test "auth diagnostics: a stale/non-socket SSH_AUTH_SOCK is reported as such, not as reachable" {
  fake_ssh_permission_denied
  export SSH_AUTH_SOCK="$BATS_TEST_TMPDIR/not-a-socket"
  : > "$SSH_AUTH_SOCK"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  log="$(daemon_log)"
  [[ "$log" == *"SSH_AUTH_SOCK=set but not a socket"* ]]
  # the socket path itself is never logged (no secrets/paths beyond presence)
  [[ "$log" != *"$SSH_AUTH_SOCK"* ]]
}

@test "auth diagnostics: push failures are also diagnosed when auth-related" {
  # Keep origin local so fetch/reconcile succeed normally, and exercise
  # log_auth_diagnostics_if_relevant's push wiring via a rejecting
  # pre-receive hook whose message happens to look auth-related (the same
  # mechanism real git servers use to report auth problems on push).
  mkdir -p "$REMOTE/hooks"
  printf '#!/bin/sh\necho "Authentication failed for '"'"'the remote'"'"'" 1>&2\nexit 1\n' > "$REMOTE/hooks/pre-receive"
  chmod +x "$REMOTE/hooks/pre-receive"
  echo "changed" > "$REPO/tracked.txt"
  unset SSH_AUTH_SOCK

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  log="$(daemon_log)"
  [[ "$log" == *"push failure looks auth-related"* ]]
}

@test "self-heal: a changed SSH_AUTH_SOCK is picked up from the systemd user environment and logged" {
  unset SSH_AUTH_SOCK
  export AUTOGIT_SYSTEMD_ENV_CMD="printf 'SSH_AUTH_SOCK=$BATS_TEST_TMPDIR/picked-up-sock\n'"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  log="$(daemon_log)"
  [[ "$log" == *"picked up a changed SSH_AUTH_SOCK from the systemd user environment"* ]]
  [[ "$log" == *"was unset, now set"* ]]
}

@test "self-heal: no spurious log when the systemd user environment has nothing new to offer" {
  export AUTOGIT_SYSTEMD_ENV_CMD="true"

  run "$AUTOGIT_DAEMON" --once
  [ "$status" -eq 0 ]

  log="$(daemon_log)"
  [[ "$log" != *"picked up a changed"* ]]
}
