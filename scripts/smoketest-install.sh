#!/usr/bin/env bash
# Smoke-test the mdagile-cli, mdagile-lsp and mdagile-gui packages by
# actually installing them (for real, no stubs/fakes) into a disposable
# Docker container, then checking that the installed binaries at least run.
#
# This intentionally does NOT touch the host system: it runs a throwaway
# container (as root, so no `sudo` is needed at all) with `dist/` bind-mounted
# read-only, installs the packages with the distro's real package manager,
# and discards the container afterwards.
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
    INSTALL_CMD='apt-get update -qq && apt-get install -y -qq /dist/mdagile-cli_'"${VERSION}_${ARCH}"'.deb /dist/mdagile-lsp_'"${VERSION}_${ARCH}"'.deb /dist/mdagile-gui_'"${VERSION}_${ARCH}"'.deb'
    UNINSTALL_CMD="apt-get remove -y mdagile-cli mdagile-lsp mdagile-gui"
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
    INSTALL_CMD='dnf install -y -q /dist/mdagile-cli-'"${VERSION}-${RELEASE}.${RPM_ARCH}"'.rpm /dist/mdagile-lsp-'"${VERSION}-${RELEASE}.${RPM_ARCH}"'.rpm /dist/mdagile-gui-'"${VERSION}-${RELEASE}.${RPM_ARCH}"'.rpm'
    UNINSTALL_CMD="dnf remove -y mdagile-cli mdagile-lsp mdagile-gui"
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
# combined pass/fail exit code back. Runs as the container's root user, so no
# sudo/privilege-escalation stub of any kind is needed.
read -r -d '' CONTAINER_SCRIPT <<'INNER_SCRIPT' || true
set -euo pipefail

echo "--- installing packages ---"
eval "$MDAGILE_INSTALL_CMD"

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

echo "--- uninstalling packages ---"
eval "$MDAGILE_UNINSTALL_CMD"

echo "--- checking binaries are gone ---"
hash -r # clear bash's command-path cache; command -v can otherwise report stale hits
if command -v agile >/dev/null 2>&1 || command -v agilels >/dev/null 2>&1 || command -v agilegui >/dev/null 2>&1; then
  echo "error: one or more mdagile binaries still present after uninstall" >&2
  exit 1
fi

echo "--- OK ---"
INNER_SCRIPT

docker run --rm \
  -v "$DIST_DIR:/dist:ro" \
  -e "MDAGILE_INSTALL_CMD=$INSTALL_CMD" \
  -e "MDAGILE_UNINSTALL_CMD=$UNINSTALL_CMD" \
  "$IMAGE" \
  bash -c "$CONTAINER_SCRIPT"

echo "=== smoketest-install: $PACKAGING_SYSTEM: PASSED ==="
