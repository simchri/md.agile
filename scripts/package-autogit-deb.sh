#!/usr/bin/env bash
# Assemble the mdagile-autogit .deb package from the bash scripts under
# autogit/ (see doc/development/autogit.md). No build step is required —
# unlike mdagile-cli/-lsp/-gui, this package doesn't come from a cargo
# build artifact, just plain files. Intended to be run inside the project's
# docker dev container (see Makefile target `package`), but works on any
# Debian/Ubuntu host with dpkg-deb available.
#
# The .rpm equivalent lives in scripts/package-rpm.sh (mdagile-autogit
# section), reusing the same autogit/packaging/{postinst,prerm} scripts via
# rpm's %include.
set -euo pipefail

VERSION="${1:?usage: package-autogit-deb.sh <version>}"
ARCH="$(dpkg --print-architecture)"
MAINTAINER="mdagile maintainers <noreply@example.invalid>"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DIST_DIR="dist"
STAGE_DIR="$DIST_DIR/stage"
PKG_DIR="$STAGE_DIR/mdagile-autogit"

for f in autogit/bin/autogit autogit/bin/autogit-daemon autogit/systemd/autogit.service \
         autogit/packaging/postinst autogit/packaging/prerm; do
  if [ ! -e "$f" ]; then
    echo "error: expected autogit source file missing: $f" >&2
    exit 1
  fi
done

mkdir -p "$DIST_DIR"
rm -rf "$PKG_DIR"
rm -f "$DIST_DIR/mdagile-autogit_${VERSION}_${ARCH}.deb"

mkdir -p \
  "$PKG_DIR/usr/bin" \
  "$PKG_DIR/usr/lib/systemd/user" \
  "$PKG_DIR/DEBIAN"

install -m 755 autogit/bin/autogit "$PKG_DIR/usr/bin/autogit"
install -m 755 autogit/bin/autogit-daemon "$PKG_DIR/usr/bin/autogit-daemon"
install -m 644 autogit/systemd/autogit.service "$PKG_DIR/usr/lib/systemd/user/autogit.service"

# Deliberately no default.target.wants/autogit.service symlink in the
# package: under /usr/lib/systemd/user/ it would enable the service for
# *every* user on the system (incl. system users like gdm). Instead,
# postinst enables it only for the user who ran the install.

cat > "$PKG_DIR/DEBIAN/control" <<EOF
Package: mdagile-autogit
Version: $VERSION
Section: devel
Priority: optional
Architecture: $ARCH
Maintainer: $MAINTAINER
Description: automatic git commit/push/pull sync daemon for mdagile (scaffolding only, no real sync behavior yet)
EOF

install -m 755 autogit/packaging/postinst "$PKG_DIR/DEBIAN/postinst"
install -m 755 autogit/packaging/prerm "$PKG_DIR/DEBIAN/prerm"

dpkg-deb --build --root-owner-group "$PKG_DIR" "$DIST_DIR/mdagile-autogit_${VERSION}_${ARCH}.deb"

echo "Built package:"
ls -1 "$DIST_DIR"/mdagile-autogit_*.deb
