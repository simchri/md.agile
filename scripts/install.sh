#!/usr/bin/env bash
#
# Builds and installs mdagile from source, on the host (no Docker / devenv
# involved — the GUI mode needs the real Rust + dx toolchain to perform the
# wasm web bundling step, so this script deliberately runs outside the
# container that the rest of the project's dev workflow uses).
#
# Three independent, individually selectable modes:
#   --toolchain   installs prerequisites needed by the other modes
#                 (wasm32-unknown-unknown target, dioxus-cli). Assumes Rust
#                 itself (cargo/rustup) is already installed.
#   --cli         builds and installs the `agile` and `agilels` binaries.
#   --gui         bundles the web assets (dx bundle), bakes them into a
#                 single self-contained server binary, and installs it.
#
# With no flags, all three modes run, in order (toolchain, then cli, then
# gui) — each mode is idempotent and can also be run standalone later, e.g.
# to only refresh the GUI after pulling new changes:
#
#   ./scripts/install.sh --gui
#
# Usage:
#   ./scripts/install.sh [--toolchain] [--cli] [--gui] [--all] [-h|--help]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_DIR="${HOME}/.local/bin"

fail() {
    echo "error: $*" >&2
    exit 1
}

print_help() {
    sed -n '2,24p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

DO_TOOLCHAIN=0
DO_CLI=0
DO_GUI=0
any_mode_specified=0

for arg in "$@"; do
    case "${arg}" in
        --toolchain) DO_TOOLCHAIN=1; any_mode_specified=1 ;;
        --cli) DO_CLI=1; any_mode_specified=1 ;;
        --gui) DO_GUI=1; any_mode_specified=1 ;;
        --all) DO_TOOLCHAIN=1; DO_CLI=1; DO_GUI=1; any_mode_specified=1 ;;
        -h|--help) print_help; exit 0 ;;
        *) fail "unknown option: ${arg} (expected --toolchain, --cli, --gui, --all, or --help)" ;;
    esac
done

# Default: no mode flags given at all -> install everything.
if [ "${any_mode_specified}" -eq 0 ]; then
    DO_TOOLCHAIN=1
    DO_CLI=1
    DO_GUI=1
fi

install_toolchain() {
    echo "==> [toolchain] Checking Rust toolchain"
    if ! command -v cargo >/dev/null 2>&1; then
        fail "cargo not found on PATH. Install Rust: https://rustup.rs"
    fi

    if command -v rustup >/dev/null 2>&1 \
        && rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
        echo "    wasm32-unknown-unknown target already installed"
    else
        if ! command -v rustup >/dev/null 2>&1; then
            fail "rustup not found on PATH, needed to install the wasm32-unknown-unknown target.
Install rustup: https://rustup.rs"
        fi
        echo "==> [toolchain] Installing wasm32-unknown-unknown target"
        rustup target add wasm32-unknown-unknown
    fi

    if command -v dx >/dev/null 2>&1; then
        echo "    dx CLI already installed ($(dx --version 2>/dev/null || true))"
    else
        echo "==> [toolchain] Installing dioxus-cli (dx)"
        cargo install dioxus-cli --locked
    fi
}

check_gui_prerequisites() {
    if ! command -v dx >/dev/null 2>&1; then
        fail "the 'dx' CLI (Dioxus CLI) was not found on PATH.
Run ./scripts/install.sh --toolchain first, or install it manually with:
  cargo install dioxus-cli --locked"
    fi

    if ! rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
        fail "the 'wasm32-unknown-unknown' target is not installed.
Run ./scripts/install.sh --toolchain first, or install it manually with:
  rustup target add wasm32-unknown-unknown"
    fi
}

install_cli() {
    echo "==> [cli] Building agile + agilels (cargo build --release -p mdagile)"
    (
        cd "${REPO_ROOT}"
        cargo build --release -p mdagile
    )

    mkdir -p "${INSTALL_DIR}"
    for bin in agile agilels; do
        local_bin="${REPO_ROOT}/target/release/${bin}"
        if [ ! -f "${local_bin}" ]; then
            fail "expected built binary not found at ${local_bin}"
        fi
        cp "${local_bin}" "${INSTALL_DIR}/${bin}"
        chmod +x "${INSTALL_DIR}/${bin}"
    done

    echo "==> [cli] Installed ${INSTALL_DIR}/agile and ${INSTALL_DIR}/agilels"
}

install_gui() {
    check_gui_prerequisites

    local gui_dir="${REPO_ROOT}/crates/gui"
    local bundled_public="${gui_dir}/.bundled-assets/public"

    echo "==> [gui] Bundling web assets (dx bundle --release)"
    (
        cd "${gui_dir}"
        dx bundle --release --platform web
    )

    local bundle_output="${REPO_ROOT}/target/dx/mdagile-gui/release/web/public"
    if [ ! -f "${bundle_output}/index.html" ]; then
        fail "expected dx bundle output not found at ${bundle_output}.
'dx bundle' may have changed its output layout; please check manually."
    fi

    echo "==> [gui] Staging bundled assets for embedding"
    rm -rf "${gui_dir}/.bundled-assets"
    mkdir -p "${gui_dir}/.bundled-assets"
    cp -r "${bundle_output}" "${bundled_public}"

    echo "==> [gui] Building self-contained server binary (cargo build --release --features embed-assets)"
    (
        cd "${REPO_ROOT}"
        cargo build --release --features embed-assets -p mdagile-gui
    )

    local built_binary="${REPO_ROOT}/target/release/mdagile-gui"
    if [ ! -f "${built_binary}" ]; then
        fail "expected built binary not found at ${built_binary}"
    fi

    mkdir -p "${INSTALL_DIR}"
    cp "${built_binary}" "${INSTALL_DIR}/agilegui"
    chmod +x "${INSTALL_DIR}/agilegui"

    echo "==> [gui] Installed ${INSTALL_DIR}/agilegui"
}

[ "${DO_TOOLCHAIN}" -eq 1 ] && install_toolchain
[ "${DO_CLI}" -eq 1 ] && install_cli
[ "${DO_GUI}" -eq 1 ] && install_gui

echo
echo "Done."
[ "${DO_CLI}" -eq 1 ] && echo "  agile / agilels installed to ${INSTALL_DIR}"
[ "${DO_GUI}" -eq 1 ] && echo "  agilegui installed to ${INSTALL_DIR} (run with MDAGILE_WORKDIR=/path/to/project agilegui)"

case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *) echo "note: ${INSTALL_DIR} is not on your PATH — add it, e.g. in ~/.bashrc:" \
        && echo "  export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
esac
