#!/usr/bin/env bash
# Assemble .rpm packages for mdagile-cli, mdagile-lsp and mdagile-gui from
# previously built release artifacts, using rpmbuild. Intended to be run
# inside the project's docker dev container (see Makefile target `package`),
# which has the `rpm` apt package installed (providing `rpmbuild`) so it can
# build .rpm packages even though the container itself is Debian-based —
# rpmbuild doesn't require an RPM-based host to produce valid .rpm archives.
set -euo pipefail

VERSION="${1:?usage: package-rpm.sh <version>}"
MAINTAINER="mdagile maintainers <noreply@example.invalid>"
RELEASE="1"
RPM_ARCH="$(uname -m)" # e.g. x86_64

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v rpmbuild >/dev/null 2>&1; then
  echo "error: rpmbuild not found (install the 'rpm' package)" >&2
  exit 1
fi

DIST_DIR="dist"
TOPDIR="$ROOT/$DIST_DIR/rpmbuild"

CLI_BIN="target/release/agile"
LSP_BIN="target/release/agilels"
GUI_WEB_DIR="target/dx/mdagile-gui/release/web"

for f in "$CLI_BIN" "$LSP_BIN" "$GUI_WEB_DIR/server" "$GUI_WEB_DIR/public"; do
  if [ ! -e "$f" ]; then
    echo "error: expected build artifact missing: $f (run 'make build-release' first)" >&2
    exit 1
  fi
done

mkdir -p "$DIST_DIR"

# Remove any stale packages from previous runs (regardless of their name,
# version or arch) so `dist/` never mixes old and freshly-built .rpms, which
# would otherwise confuse subsequent `make install` steps.
rm -rf "$TOPDIR"
mkdir -p "$TOPDIR"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
find "$DIST_DIR" -maxdepth 1 -name '*.rpm' -delete

write_spec() {
  local name="$1" summary="$2" install_script="$3" files="$4"
  cat > "$TOPDIR/SPECS/$name.spec" <<EOF
Name: $name
Version: $VERSION
Release: $RELEASE
Summary: $summary
License: Proprietary
BuildArch: $RPM_ARCH
Packager: $MAINTAINER

%description
$summary

%install
$install_script

%files
$files
EOF
}

build_rpm() {
  local name="$1"
  rpmbuild --define "_topdir $TOPDIR" -bb "$TOPDIR/SPECS/$name.spec"
  find "$TOPDIR/RPMS" -name "${name}-${VERSION}-${RELEASE}.*.rpm" -exec mv {} "$DIST_DIR/" \;
}

# --- mdagile-cli: the `agile` CLI binary ---
write_spec "mdagile-cli" \
  "mdagile command-line tool (agile) for .agile.md task files" \
  "mkdir -p %{buildroot}/usr/bin
install -m 755 $ROOT/$CLI_BIN %{buildroot}/usr/bin/agile" \
  "/usr/bin/agile"
build_rpm "mdagile-cli"

# --- mdagile-lsp: the `agilels` language server binary ---
write_spec "mdagile-lsp" \
  "Language Server Protocol implementation for mdagile (.agile.md) files" \
  "mkdir -p %{buildroot}/usr/bin
install -m 755 $ROOT/$LSP_BIN %{buildroot}/usr/bin/agilels" \
  "/usr/bin/agilels"
build_rpm "mdagile-lsp"

# --- mdagile-gui: the dioxus fullstack web server + static assets ---
write_spec "mdagile-gui" \
  "mdagile board viewer (web GUI server) for .agile.md task files" \
  "mkdir -p %{buildroot}/usr/lib/mdagile-gui %{buildroot}/usr/bin %{buildroot}/usr/share/applications
install -m 755 $ROOT/$GUI_WEB_DIR/server %{buildroot}/usr/lib/mdagile-gui/server
cp -r $ROOT/$GUI_WEB_DIR/public %{buildroot}/usr/lib/mdagile-gui/public
install -m 755 $ROOT/scripts/assets/agilegui-wrapper.sh %{buildroot}/usr/bin/agilegui
install -m 644 $ROOT/scripts/assets/mdagile-gui.desktop %{buildroot}/usr/share/applications/mdagile-gui.desktop
install -m 644 $ROOT/scripts/assets/mdagile-gui-stop.desktop %{buildroot}/usr/share/applications/mdagile-gui-stop.desktop" \
  "/usr/lib/mdagile-gui
/usr/bin/agilegui
/usr/share/applications/mdagile-gui.desktop
/usr/share/applications/mdagile-gui-stop.desktop"
build_rpm "mdagile-gui"

rm -rf "$TOPDIR"

echo "Built packages:"
ls -1 "$DIST_DIR"/*.rpm
