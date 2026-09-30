#!/usr/bin/env bash
# Assemble the mdagile-autogit .deb package from the bash scripts under
# autogit/ (see doc/development/autogit.md). No build step is required —
# unlike mdagile-cli/-lsp/-gui, this package doesn't come from a cargo
# build artifact, just plain files. Intended to be run inside the project's
# docker dev container (see Makefile target `package`), but works on any
# Debian/Ubuntu host with dpkg-deb available.
#
# Debian only (no .rpm equivalent): the Architecture section of the design
# doc explicitly scopes autogit to a Debian package.
set -euo pipefail

VERSION="${1:?usage: package-autogit-deb.sh <version>}"
ARCH="$(dpkg --print-architecture)"
MAINTAINER="mdagile maintainers <noreply@example.invalid>"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DIST_DIR="dist"
STAGE_DIR="$DIST_DIR/stage"
PKG_DIR="$STAGE_DIR/mdagile-autogit"

for f in autogit/bin/autogit autogit/bin/autogit-daemon autogit/systemd/autogit.service; do
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
  "$PKG_DIR/usr/lib/systemd/user/default.target.wants" \
  "$PKG_DIR/DEBIAN"

install -m 755 autogit/bin/autogit "$PKG_DIR/usr/bin/autogit"
install -m 755 autogit/bin/autogit-daemon "$PKG_DIR/usr/bin/autogit-daemon"
install -m 644 autogit/systemd/autogit.service "$PKG_DIR/usr/lib/systemd/user/autogit.service"

# Ship the "enabled" symlink directly in the package, rather than creating it
# in postinst: this is what `systemctl --user enable` itself does under the
# hood, and shipping it as a plain file in the package means the unit starts
# automatically the next time *any* user logs in (systemd discovers
# default.target.wants/* when it starts that user's manager), without
# needing a live user session at package-install time.
ln -s ../autogit.service "$PKG_DIR/usr/lib/systemd/user/default.target.wants/autogit.service"

cat > "$PKG_DIR/DEBIAN/control" <<EOF
Package: mdagile-autogit
Version: $VERSION
Section: devel
Priority: optional
Architecture: $ARCH
Maintainer: $MAINTAINER
Description: automatic git commit/push/pull sync daemon for mdagile (scaffolding only, no real sync behavior yet)
EOF

# Best-effort: also start the service *now*, for whichever user actually ran
# the install, instead of only waiting for their next login (the symlink
# above already guarantees that fallback). If there's no live user session
# to talk to yet (e.g. non-interactive install), this silently falls back
# to "starts on next login" rather than failing the package install.
cat > "$PKG_DIR/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e

target_user="${SUDO_USER:-$(logname 2>/dev/null || true)}"

if [ -n "$target_user" ] && [ "$target_user" != "root" ]; then
  target_uid="$(id -u "$target_user" 2>/dev/null || true)"
  if [ -n "$target_uid" ] && su - "$target_user" -c \
      "XDG_RUNTIME_DIR=/run/user/$target_uid systemctl --user daemon-reload && XDG_RUNTIME_DIR=/run/user/$target_uid systemctl --user enable --now autogit.service" \
      >/dev/null 2>&1; then
    echo "mdagile-autogit: started now for user '$target_user'."
  else
    echo "mdagile-autogit: could not start immediately (no active user session?); it will start automatically on next login."
  fi
else
  echo "mdagile-autogit: installed. The service will start automatically on next user login."
fi

exit 0
EOF
chmod 755 "$PKG_DIR/DEBIAN/postinst"

# Best-effort: stop the running user service before removal, so an
# uninstall doesn't leave a daemon running with its unit file gone.
cat > "$PKG_DIR/DEBIAN/prerm" <<'EOF'
#!/bin/sh
set -e

target_user="${SUDO_USER:-$(logname 2>/dev/null || true)}"

if [ -n "$target_user" ] && [ "$target_user" != "root" ]; then
  target_uid="$(id -u "$target_user" 2>/dev/null || true)"
  if [ -n "$target_uid" ]; then
    su - "$target_user" -c \
      "XDG_RUNTIME_DIR=/run/user/$target_uid systemctl --user stop autogit.service" \
      >/dev/null 2>&1 || true
  fi
fi

exit 0
EOF
chmod 755 "$PKG_DIR/DEBIAN/prerm"

dpkg-deb --build --root-owner-group "$PKG_DIR" "$DIST_DIR/mdagile-autogit_${VERSION}_${ARCH}.deb"

echo "Built package:"
ls -1 "$DIST_DIR"/mdagile-autogit_*.deb
