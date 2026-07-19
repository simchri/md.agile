#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/plot-demo-common.bash
source "$SCRIPT_DIR/lib/plot-demo-common.bash"

build_agile_cli

"$SCRIPT_DIR/plot-demo-messy-both.bash" --no-build
"$SCRIPT_DIR/plot-demo-new-milestone.bash" --no-build
