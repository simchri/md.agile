#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/plot-demo-common.bash
source "$SCRIPT_DIR/lib/plot-demo-common.bash"

TARGET_DIR="sample_dir_plot_messy_both"

create_sample_repo() {
    local target_dir="$1"

    cd "$REPO_ROOT"
    init_sample_repo "$target_dir"

    compute_dates 5 14

    # --- snapshot 1: total=15 done=3 ---
    cat > tasks.agile.md <<'EOF'
- [x] Task 1
- [x] Task 2
- [x] Task 3
- [ ] Task 4
- [ ] Task 5
- [ ] Task 6
- [ ] Task 7
- [ ] Task 8
- [ ] Task 9
- [ ] Task 10
- [ ] Task 11
- [ ] Task 12
- [ ] Task 13
- [ ] Task 14
- [ ] Task 15

#MILESTONE: Demo milestone

- [ ] Task 16
EOF
    commit_snapshot 1 "${dates[0]}"

    # --- snapshot 2: total=14 done=6 ---
    cat > tasks.agile.md <<'EOF'
- [x] Task 1
- [x] Task 2
- [x] Task 3
- [x] Task 4
- [x] Task 5
- [x] Task 6
- [ ] Task 7
- [ ] Task 8
- [ ] Task 9
- [ ] Task 10
- [ ] Task 11
- [ ] Task 12
- [ ] Task 13
- [ ] Task 14

#MILESTONE: Demo milestone

- [ ] Task 15
EOF
    commit_snapshot 2 "${dates[1]}"

    # --- snapshot 3: total=16 done=8 ---
    cat > tasks.agile.md <<'EOF'
- [x] Task 1
- [x] Task 2
- [x] Task 3
- [x] Task 4
- [x] Task 5
- [x] Task 6
- [x] Task 7
- [x] Task 8
- [ ] Task 9
- [ ] Task 10
- [ ] Task 11
- [ ] Task 12
- [ ] Task 13
- [ ] Task 14
- [ ] Task 15
- [ ] Task 16

#MILESTONE: Demo milestone

- [ ] Task 17
EOF
    commit_snapshot 3 "${dates[2]}"

    # --- snapshot 4: total=17 done=10 ---
    cat > tasks.agile.md <<'EOF'
- [x] Task 1
- [x] Task 2
- [x] Task 3
- [x] Task 4
- [x] Task 5
- [x] Task 6
- [x] Task 7
- [x] Task 8
- [x] Task 9
- [x] Task 10
- [ ] Task 11
- [ ] Task 12
- [ ] Task 13
- [ ] Task 14
- [ ] Task 15
- [ ] Task 16
- [ ] Task 17

#MILESTONE: Demo milestone

- [ ] Task 18
EOF
    commit_snapshot 4 "${dates[3]}"

    # --- snapshot 5: total=18 done=12 (last commit: today) ---
    cat > tasks.agile.md <<'EOF'
- [x] Task 1
- [x] Task 2
- [x] Task 3
- [x] Task 4
- [x] Task 5
- [x] Task 6
- [x] Task 7
- [x] Task 8
- [x] Task 9
- [x] Task 10
- [x] Task 11
- [x] Task 12
- [ ] Task 13
- [ ] Task 14
- [ ] Task 15
- [ ] Task 16
- [ ] Task 17
- [ ] Task 18

#MILESTONE: Demo milestone

- [ ] Task 19
EOF
    commit_snapshot 5 "${dates[4]}"

    cd "$REPO_ROOT"
}

if [[ "${1:-}" != "--no-build" ]]; then
    build_agile_cli
fi

echo "Recreating sample repo: $TARGET_DIR (mode=messy-both)"
create_sample_repo "$TARGET_DIR"
run_plot "$TARGET_DIR"
