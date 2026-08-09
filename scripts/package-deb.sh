#!/usr/bin/env bash
# Assemble .deb packages for mdagile-cli, mdagile-lsp and mdagile-gui from
# previously built release artifacts. Intended to be run inside the project's
# docker dev container (see Makefile target `package`), but works on any
# Debian/Ubuntu host with dpkg-deb available.
set -euo pipefail

VERSION="${1:?usage: package-deb.sh <version>}"
# Detect the *build* platform's Debian architecture name (e.g. amd64, arm64)
# directly from dpkg, rather than having the caller pass it in.
ARCH="$(dpkg --print-architecture)"
MAINTAINER="mdagile maintainers <noreply@example.invalid>"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DIST_DIR="dist"
STAGE_DIR="$DIST_DIR/stage"

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
# version or arch) so `dist/` never mixes old and freshly-built .debs, which
# would otherwise confuse subsequent `make install` steps.
rm -rf "$STAGE_DIR"
find "$DIST_DIR" -maxdepth 1 -name '*.deb' -delete

write_control() {
  local dir="$1" pkg="$2" desc="$3"
  mkdir -p "$dir/DEBIAN"
  cat > "$dir/DEBIAN/control" <<EOF
Package: $pkg
Version: $VERSION
Section: devel
Priority: optional
Architecture: $ARCH
Maintainer: $MAINTAINER
Description: $desc
EOF
}

build_deb() {
  local name="$1"
  dpkg-deb --build --root-owner-group "$STAGE_DIR/$name" "$DIST_DIR/${name}_${VERSION}_${ARCH}.deb"
}

# --- mdagile-cli: the `agile` CLI binary ---
CLI_DIR="$STAGE_DIR/mdagile-cli"
mkdir -p "$CLI_DIR/usr/bin"
install -m 755 "$CLI_BIN" "$CLI_DIR/usr/bin/agile"
write_control "$CLI_DIR" "mdagile-cli" "mdagile command-line tool (agile) for .agile.md task files"
build_deb "mdagile-cli"

# --- mdagile-lsp: the `agilels` language server binary ---
LSP_DIR="$STAGE_DIR/mdagile-lsp"
mkdir -p "$LSP_DIR/usr/bin"
install -m 755 "$LSP_BIN" "$LSP_DIR/usr/bin/agilels"
write_control "$LSP_DIR" "mdagile-lsp" "Language Server Protocol implementation for mdagile (.agile.md) files"
build_deb "mdagile-lsp"

# --- mdagile-gui: the dioxus fullstack web server + static assets ---
GUI_DIR="$STAGE_DIR/mdagile-gui"
mkdir -p "$GUI_DIR/usr/lib/mdagile-gui" "$GUI_DIR/usr/bin" "$GUI_DIR/usr/share/applications"
install -m 755 "$GUI_WEB_DIR/server" "$GUI_DIR/usr/lib/mdagile-gui/server"
cp -r "$GUI_WEB_DIR/public" "$GUI_DIR/usr/lib/mdagile-gui/public"
install -m 755 "$ROOT/scripts/assets/agilegui-wrapper.sh" "$GUI_DIR/usr/bin/agilegui"
# App-menu entries: one to launch the GUI (one-click), one to stop it —
# both just invoke `agilegui`/`agilegui stop` so users don't need a
# terminal for the common cases.
install -m 644 "$ROOT/scripts/assets/mdagile-gui.desktop" "$GUI_DIR/usr/share/applications/mdagile-gui.desktop"
install -m 644 "$ROOT/scripts/assets/mdagile-gui-stop.desktop" "$GUI_DIR/usr/share/applications/mdagile-gui-stop.desktop"
write_control "$GUI_DIR" "mdagile-gui" "mdagile board viewer (web GUI server) for .agile.md task files"
build_deb "mdagile-gui"

echo "Built packages:"
ls -1 "$DIST_DIR"/*.deb
