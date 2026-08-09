#!/usr/bin/env bash
# Install the previously built mdagile-cli, mdagile-lsp and mdagile-gui .deb
# packages (see scripts/package-deb.sh / `make package`) onto the *host*
# system. Runs on the host (not in the docker dev container), since it needs
# to modify the host's package database.
set -euo pipefail

VERSION="${1:?usage: install.sh <version>}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Re-run host packaging-system detection (rather than trusting a value/file
# handed down from `make`) so this script stays self-contained and correct
# even when invoked directly, outside of `make install`.
PACKAGING_SYSTEM="$(scripts/detect-packaging-system.sh value)"

# Must be an absolute path: a bare "dist/pkg.deb" (no leading "./" or "/") is
# ambiguous with apt-get's "package/release" target-release syntax, which
# makes apt-get misinterpret "dist" as a release name instead of a directory.
DIST_DIR="$ROOT/dist"

case "$PACKAGING_SYSTEM" in
  debian)
    # Detect the host's Debian architecture name (e.g. amd64, arm64) directly
    # from dpkg, so it always matches whatever package-deb.sh produced.
    ARCH="$(dpkg --print-architecture)"
    PACKAGES=(
      "$DIST_DIR/mdagile-cli_${VERSION}_${ARCH}.deb"
      "$DIST_DIR/mdagile-lsp_${VERSION}_${ARCH}.deb"
      "$DIST_DIR/mdagile-gui_${VERSION}_${ARCH}.deb"
    )
    # `--reinstall` so re-running `make install` after rebuilding a package
    # with the *same* version number (e.g. local dev iteration, no version
    # bump) actually reinstalls the files instead of apt-get silently
    # no-op'ing because it thinks the "already installed" version satisfies
    # the request — otherwise `make install` doesn't behave like an "update"
    # command, which is surprising.
    cmd=(sudo apt-get install --reinstall -y "${PACKAGES[@]}")
    uninstall_cmd="sudo apt-get remove -y mdagile-cli mdagile-lsp mdagile-gui"
    ;;
  rpm)
    RPM_ARCH="$(uname -m)"
    RELEASE="1"
    PACKAGES=(
      "$DIST_DIR/mdagile-cli-${VERSION}-${RELEASE}.${RPM_ARCH}.rpm"
      "$DIST_DIR/mdagile-lsp-${VERSION}-${RELEASE}.${RPM_ARCH}.rpm"
      "$DIST_DIR/mdagile-gui-${VERSION}-${RELEASE}.${RPM_ARCH}.rpm"
    )
    if command -v dnf >/dev/null 2>&1; then
      # `dnf reinstall` (unlike apt's `--reinstall`) errors out if the
      # package isn't already installed at all, so first-time installs still
      # need plain `install`. Pick the right one up front (rather than
      # printing one command below and silently running another) based on
      # whether any of the three packages are already present.
      if rpm -q mdagile-cli mdagile-lsp mdagile-gui >/dev/null 2>&1; then
        cmd=(sudo dnf reinstall -y "${PACKAGES[@]}")
      else
        cmd=(sudo dnf install -y "${PACKAGES[@]}")
      fi
      uninstall_cmd="sudo dnf remove -y mdagile-cli mdagile-lsp mdagile-gui"
    elif command -v yum >/dev/null 2>&1; then
      if rpm -q mdagile-cli mdagile-lsp mdagile-gui >/dev/null 2>&1; then
        cmd=(sudo yum reinstall -y "${PACKAGES[@]}")
      else
        cmd=(sudo yum install -y "${PACKAGES[@]}")
      fi
      uninstall_cmd="sudo yum remove -y mdagile-cli mdagile-lsp mdagile-gui"
    else
      echo "error: neither dnf nor yum found on this host" >&2
      exit 1
    fi
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
echo "Installed successfully."
echo "You can uninstall md.agile as follows:"
echo ""
echo "  $uninstall_cmd"
echo ""
echo "-----------------------------------------------------------------"
