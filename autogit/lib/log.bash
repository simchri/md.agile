#!/usr/bin/env bash
# Shared logging helper for autogit scripts (the `autogit` CLI and the
# systemd daemon each source this and log through the same mechanism, into
# separate per-component files — see doc/development/autogit.md "logging").
#
# Scaffolding scope: writes one file per component per day. Weekly rotation
# (per the design doc) is intentionally not implemented yet; that's a
# separate, later task.

AUTOGIT_LOG_DIR="${AUTOGIT_LOG_DIR:-/tmp/autogit}"

# autogit_log <component> <message>
autogit_log() {
  local component="$1"
  local message="$2"
  mkdir -p "$AUTOGIT_LOG_DIR"
  local file="$AUTOGIT_LOG_DIR/${component}-$(date +%F).log"
  printf '%s [%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$component" "$message" >> "$file"
}
