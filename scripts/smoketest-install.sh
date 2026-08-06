#!/usr/bin/env bash
# Smoke-test the mdagile-cli, mdagile-lsp and mdagile-gui packages by
# actually installing them (for real, no stubs/fakes) into a disposable
# Docker container, then checking that the installed binaries at least run.
#
# This intentionally does NOT touch the host system: it runs a throwaway
# container with the whole repo bind-mounted read-only, and calls the real
# scripts/install.sh *inside* that container (as root) — the same script a
# user would run on their own machine — rather than duplicating its
# install-command logic here. This keeps this smoke test honest: it exercises
# exactly what users actually run, and stays in sync with install.sh
# automatically instead of needing separate maintenance.
#
# Usage: smoketest-install.sh <packaging-system> <version>
#   packaging-system: debian | rpm
set -euo pipefail

PACKAGING_SYSTEM="${1:?usage: smoketest-install.sh <packaging-system> <version>}"
VERSION="${2:?usage: smoketest-install.sh <packaging-system> <version>}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="$ROOT/dist"

case "$PACKAGING_SYSTEM" in
  debian)
    # Must match (or be newer than) the glibc of the dev/build image
    # (rustlang/rust "trixie" base, i.e. Debian 13) — an older base like
    # bookworm (Debian 12) has an older glibc than the binaries were linked
    # against, and fails with "GLIBC_2.39' not found".
    IMAGE="debian:trixie-slim"
    ARCH="$(dpkg --print-architecture 2>/dev/null || echo amd64)"
    PACKAGE_FILES=(
      "mdagile-cli_${VERSION}_${ARCH}.deb"
      "mdagile-lsp_${VERSION}_${ARCH}.deb"
      "mdagile-gui_${VERSION}_${ARCH}.deb"
    )
    # install.sh shells out to the real `sudo`, so it needs to exist in the
    # container; running it as root is passwordless (no PAM prompt), so this
    # stays fully non-interactive.
    PREP_CMD="apt-get update -qq && apt-get install -y -qq sudo"
    ;;
  rpm)
    IMAGE="fedora:latest"
    RPM_ARCH="$(uname -m)"
    RELEASE="1"
    PACKAGE_FILES=(
      "mdagile-cli-${VERSION}-${RELEASE}.${RPM_ARCH}.rpm"
      "mdagile-lsp-${VERSION}-${RELEASE}.${RPM_ARCH}.rpm"
      "mdagile-gui-${VERSION}-${RELEASE}.${RPM_ARCH}.rpm"
    )
    PREP_CMD="dnf install -y -q sudo"
    ;;
  *)
    echo "error: unsupported packaging system '$PACKAGING_SYSTEM' (expected debian or rpm)" >&2
    exit 1
    ;;
esac

for f in "${PACKAGE_FILES[@]}"; do
  if [ ! -e "$DIST_DIR/$f" ]; then
    echo "error: expected package missing: $DIST_DIR/$f (run 'make package' first)" >&2
    exit 1
  fi
done

echo "=== smoketest-install: $PACKAGING_SYSTEM ($IMAGE) ==="

# The whole check runs as a single script inside the container so we get one
# combined pass/fail exit code back. Runs as the container's root user, so
# `sudo` (installed via $MDAGILE_PREP_CMD above) works passwordlessly.
read -r -d '' CONTAINER_SCRIPT <<'INNER_SCRIPT' || true
set -euo pipefail

echo "--- preparing container (installing sudo) ---"
eval "$MDAGILE_PREP_CMD"

echo "--- running the real install.sh ---"
install_output="$(/repo/scripts/install.sh "$MDAGILE_VERSION" 2>&1)"
echo "$install_output"

echo "--- checking agile (cli) ---"
agile --help >/dev/null

echo "--- checking agilels (lsp) ---"
agilels --help >/dev/null || true # LSP binaries often don't support --help; presence + exec is enough
command -v agilels >/dev/null

echo "--- checking agilegui (gui) doesn't immediately exit ---"
workdir="$(mktemp -d)"
cat > "$workdir/mdagile.toml" <<'EOF'
[Properties]
[Users]
[Groups]
EOF
MDAGILE_WORKDIR="$workdir" agilegui &
gui_pid=$!
sleep 1
if ! kill -0 "$gui_pid" 2>/dev/null; then
  echo "error: agilegui exited prematurely within 1s" >&2
  exit 1
fi
kill "$gui_pid" 2>/dev/null || true
wait "$gui_pid" 2>/dev/null || true
echo "agilegui was still running after 1s (as expected)"

# Extract the exact uninstall command install.sh printed in its
# post-install summary, rather than duplicating that logic here — this way
# the smoke test always uninstalls with precisely what a real user was told
# to run, and stays correct even if install.sh's uninstall command changes.
uninstall_cmd="$(echo "$install_output" | grep -E '^\s*sudo (apt-get|dnf|yum) remove ' | sed -E 's/^\s+//')"
if [ -z "$uninstall_cmd" ]; then
  echo "error: could not find uninstall command in install.sh output" >&2
  exit 1
fi

echo "--- uninstalling packages via: $uninstall_cmd ---"
eval "$uninstall_cmd"

echo "--- checking binaries are gone ---"
hash -r # clear bash's command-path cache; command -v can otherwise report stale hits
if command -v agile >/dev/null 2>&1 || command -v agilels >/dev/null 2>&1 || command -v agilegui >/dev/null 2>&1; then
  echo "error: one or more mdagile binaries still present after uninstall" >&2
  exit 1
fi

echo "--- OK ---"
INNER_SCRIPT

docker run --rm \
  -v "$ROOT:/repo:ro" \
  -e "MDAGILE_PREP_CMD=$PREP_CMD" \
  -e "MDAGILE_VERSION=$VERSION" \
  "$IMAGE" \
  bash -c "$CONTAINER_SCRIPT"

echo "=== smoketest-install: $PACKAGING_SYSTEM: PASSED ==="
