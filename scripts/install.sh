#!/usr/bin/env bash
# Install the previously built mdagile-cli, mdagile-lsp and mdagile-gui .deb
# packages (see scripts/package-deb.sh / `make package`) onto the *host*
# system. Runs on the host (not in the docker dev container), since it needs
# to modify the host's package database.
set -euo pipefail

VERSION="${1:?usage: install.sh <version> <arch>}"
ARCH="${2:-amd64}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Re-run host packaging-system detection (rather than trusting a value/file
# handed down from `make`) so this script stays self-contained and correct
# even when invoked directly, outside of `make install`.
PACKAGING_SYSTEM="$(scripts/detect-packaging-system.sh value)"

DIST_DIR="dist"
PACKAGES=(
  "$DIST_DIR/mdagile-cli_${VERSION}_${ARCH}.deb"
  "$DIST_DIR/mdagile-lsp_${VERSION}_${ARCH}.deb"
  "$DIST_DIR/mdagile-gui_${VERSION}_${ARCH}.deb"
)

case "$PACKAGING_SYSTEM" in
  debian)
    cmd=(sudo apt-get install -y "${PACKAGES[@]}")
    ;;
  *)
    echo "error: unsupported packaging system '$PACKAGING_SYSTEM'" >&2
    exit 1
    ;;
esac

echo "-----------------------------------------------------------------"
echo "About to install mdagile-cli, mdagile-lsp and mdagile-gui system-wide."
echo "This requires root privileges (to write into /usr/bin, /usr/lib), so"
echo "sudo will prompt you for your password. The exact command about to"
echo "be run is:"
echo ""
echo "  ${cmd[*]}"
echo ""
echo "-----------------------------------------------------------------"

"${cmd[@]}"

echo ""
echo "-----------------------------------------------------------------"
echo "Installed mdagile-cli, mdagile-lsp and mdagile-gui successfully."
echo "You can uninstall md.agile as follows:"
echo ""
echo "  sudo apt-get remove -y mdagile-cli mdagile-lsp mdagile-gui"
echo ""
echo "-----------------------------------------------------------------"
