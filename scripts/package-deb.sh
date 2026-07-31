#!/usr/bin/env bash
# Assemble .deb packages for mdagile-cli, mdagile-lsp and mdagile-gui from
# previously built release artifacts. Intended to be run inside the project's
# docker dev container (see Makefile target `package`), but works on any
# Debian/Ubuntu host with dpkg-deb available.
set -euo pipefail

VERSION="${1:?usage: package-deb.sh <version> <arch>}"
ARCH="${2:-amd64}"
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

rm -rf "$STAGE_DIR"
mkdir -p "$DIST_DIR"

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
mkdir -p "$GUI_DIR/usr/lib/mdagile-gui" "$GUI_DIR/usr/bin"
install -m 755 "$GUI_WEB_DIR/server" "$GUI_DIR/usr/lib/mdagile-gui/server"
cp -r "$GUI_WEB_DIR/public" "$GUI_DIR/usr/lib/mdagile-gui/public"
cat > "$GUI_DIR/usr/bin/agilegui" <<'WRAPPER'
#!/bin/sh
# The bundled server resolves its static assets (public/) relative to its
# working directory, so cd into the install dir before exec-ing it.
cd /usr/lib/mdagile-gui || exit 1
exec ./server "$@"
WRAPPER
chmod 755 "$GUI_DIR/usr/bin/agilegui"
write_control "$GUI_DIR" "mdagile-gui" "mdagile board viewer (web GUI server) for .agile.md task files"
build_deb "mdagile-gui"

echo "Built packages:"
ls -1 "$DIST_DIR"/*.deb
