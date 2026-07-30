#!/usr/bin/env bash
#
# Builds and installs the mdagile GUI from source, on the host (no Docker /
# devenv involved — this script needs the real Rust + dx toolchain to
# perform the wasm web bundling step, so it deliberately runs outside the
# container that the rest of the project's dev workflow uses).
#
# What it does:
#   1. Checks prerequisites (`dx` CLI, `wasm32-unknown-unknown` target).
#   2. Runs `dx bundle --release` to produce the web assets (wasm/js/css).
#   3. Copies those assets into crates/gui/.bundled-assets/public (gitignored).
#   4. Builds a self-contained server binary with the assets embedded
#      (`cargo build --release --features embed-assets`) — a fresh top-level
#      cargo invocation, not nested inside another build, so there's no risk
#      of recursive/conflicting cargo or dx invocations.
#   5. Installs the resulting single binary to ~/.local/bin/agilegui.
#
# Usage:
#   ./scripts/install-gui.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUI_DIR="${REPO_ROOT}/crates/gui"
BUNDLED_PUBLIC="${GUI_DIR}/.bundled-assets/public"
INSTALL_DIR="${HOME}/.local/bin"
INSTALL_NAME="agilegui"

fail() {
    echo "error: $*" >&2
    exit 1
}

echo "==> Checking prerequisites"

if ! command -v cargo >/dev/null 2>&1; then
    fail "cargo not found on PATH. Install Rust: https://rustup.rs"
fi

if ! command -v dx >/dev/null 2>&1; then
    fail "the 'dx' CLI (Dioxus CLI) was not found on PATH.
Install it with:  cargo install dioxus-cli --locked
(see https://dioxuslabs.com/learn/0.7/getting_started for details)"
fi

if ! rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
    fail "the 'wasm32-unknown-unknown' target is not installed.
Install it with:  rustup target add wasm32-unknown-unknown"
fi

echo "==> Bundling web assets (dx bundle --release)"
(
    cd "${GUI_DIR}"
    dx bundle --release --platform web
)

BUNDLE_OUTPUT="${REPO_ROOT}/target/dx/mdagile-gui/release/web/public"
if [ ! -f "${BUNDLE_OUTPUT}/index.html" ]; then
    fail "expected dx bundle output not found at ${BUNDLE_OUTPUT}.
'dx bundle' may have changed its output layout; please check manually."
fi

echo "==> Staging bundled assets for embedding"
rm -rf "${GUI_DIR}/.bundled-assets"
mkdir -p "${GUI_DIR}/.bundled-assets"
cp -r "${BUNDLE_OUTPUT}" "${BUNDLED_PUBLIC}"

echo "==> Building self-contained server binary (cargo build --release --features embed-assets)"
(
    cd "${REPO_ROOT}"
    cargo build --release --features embed-assets -p mdagile-gui
)

BUILT_BINARY="${REPO_ROOT}/target/release/mdagile-gui"
if [ ! -f "${BUILT_BINARY}" ]; then
    fail "expected built binary not found at ${BUILT_BINARY}"
fi

echo "==> Installing to ${INSTALL_DIR}/${INSTALL_NAME}"
mkdir -p "${INSTALL_DIR}"
cp "${BUILT_BINARY}" "${INSTALL_DIR}/${INSTALL_NAME}"
chmod +x "${INSTALL_DIR}/${INSTALL_NAME}"

echo
echo "Done. Run the GUI with:"
echo "  MDAGILE_WORKDIR=/path/to/your/project ${INSTALL_DIR}/${INSTALL_NAME}"
echo
case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *) echo "note: ${INSTALL_DIR} is not on your PATH — add it, e.g. in ~/.bashrc:" \
        && echo "  export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
esac
