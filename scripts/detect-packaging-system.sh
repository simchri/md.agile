#!/usr/bin/env bash
# Detect the *host's* (not the docker container's) packaging system.
#
# Modes (first argument):
#   check (default) - verbose: print a human-friendly detection message and
#                      exit 0/1. Used directly as the `make detect-packaging-system`
#                      target so unsupported hosts fail fast, before any
#                      build/package work happens.
#   value            - quiet: print only the detected packaging system name
#                      (e.g. "debian") to stdout, nothing else. Still exits
#                      non-zero with an error on stderr if unsupported. Used
#                      by scripts/install.sh to (re-)run detection and
#                      capture the result via command substitution.
set -euo pipefail

MODE="${1:-check}"

detect() {
  if [ -r /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    case " ${ID:-} ${ID_LIKE:-} " in
      *" debian "*) echo "debian"; return 0 ;;
      # Fedora/RHEL family (Fedora, RHEL, CentOS, Rocky, AlmaLinux, ...), all
      # using dnf (or, on older releases, yum) and .rpm packages.
      *" fedora "*|*" rhel "*) echo "rpm"; return 0 ;;
    esac
  fi
  return 1
}

if ! system="$(detect)"; then
  echo "error: unsupported host packaging system (only Debian/Ubuntu-family hosts (apt/dpkg) and Fedora/RHEL-family hosts (dnf/rpm) are currently supported)" >&2
  exit 1
fi

case "$MODE" in
  check)
    echo "Detected host packaging system: $system"
    ;;
  value)
    echo "$system"
    ;;
  *)
    echo "error: unknown mode '$MODE' (expected 'check' or 'value')" >&2
    exit 1
    ;;
esac
