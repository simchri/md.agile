#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/plot-demo-common.bash
source "$SCRIPT_DIR/lib/plot-demo-common.bash"

TARGET_DIR="sample_dir_plot_new_milestone"

create_sample_repo() {
    local target_dir="$1"

    cd "$REPO_ROOT"
    init_sample_repo "$target_dir"

    compute_dates 5 14

    # --- snapshot 1: total=15 done=3 (same as messy-both) ---
    cat > tasks.agile.md <<'EOF'
- [x] Stabilize parser edge-cases
- [x] Improve command help text
- [x] Refine ETA baseline
- [ ] Harden config loading
- [ ] Optimize task discovery
- [ ] Polish history output
- [ ] Tune velocity windowing
- [ ] Add milestone summaries
- [ ] Reduce startup overhead
- [ ] Improve marker diagnostics
- [ ] Refactor task traversal
- [ ] Document CLI examples
- [ ] Improve cache invalidation notes
- [ ] Polish release checklist
- [ ] Generated sample task 15

#MILESTONE: Demo milestone

- [ ] Post-milestone follow-up
EOF
    commit_snapshot 1 "${dates[0]}"

    # --- snapshot 2: total=14 done=6 (same as messy-both) ---
    cat > tasks.agile.md <<'EOF'
- [x] Stabilize parser edge-cases
- [x] Improve command help text
- [x] Refine ETA baseline
- [x] Harden config loading
- [x] Optimize task discovery
- [x] Polish history output
- [ ] Tune velocity windowing
- [ ] Add milestone summaries
- [ ] Reduce startup overhead
- [ ] Improve marker diagnostics
- [ ] Refactor task traversal
- [ ] Document CLI examples
- [ ] Improve cache invalidation notes
- [ ] Polish release checklist

#MILESTONE: Demo milestone

- [ ] Post-milestone follow-up
EOF
    commit_snapshot 2 "${dates[1]}"

    # --- snapshot 3: total=16 done=8 (same as messy-both) ---
    cat > tasks.agile.md <<'EOF'
- [x] Stabilize parser edge-cases
- [x] Improve command help text
- [x] Refine ETA baseline
- [x] Harden config loading
- [x] Optimize task discovery
- [x] Polish history output
- [x] Tune velocity windowing
- [x] Add milestone summaries
- [ ] Reduce startup overhead
- [ ] Improve marker diagnostics
- [ ] Refactor task traversal
- [ ] Document CLI examples
- [ ] Improve cache invalidation notes
- [ ] Polish release checklist
- [ ] Generated sample task 15
- [ ] Generated sample task 16

#MILESTONE: Demo milestone

- [ ] Post-milestone follow-up
EOF
    commit_snapshot 3 "${dates[2]}"

    # --- snapshot 4: total=17 done=10, plus a new milestone appended at the end ---
    cat > tasks.agile.md <<'EOF'
- [x] Stabilize parser edge-cases
- [x] Improve command help text
- [x] Refine ETA baseline
- [x] Harden config loading
- [x] Optimize task discovery
- [x] Polish history output
- [x] Tune velocity windowing
- [x] Add milestone summaries
- [x] Reduce startup overhead
- [x] Improve marker diagnostics
- [ ] Refactor task traversal
- [ ] Document CLI examples
- [ ] Improve cache invalidation notes
- [ ] Polish release checklist
- [ ] Generated sample task 15
- [ ] Generated sample task 16
- [ ] Generated sample task 17

#MILESTONE: Demo milestone

- [ ] Post-milestone follow-up

#MILESTONE: Second milestone

- [ ] Post-second-milestone follow-up
EOF
    commit_snapshot 4 "${dates[3]}"

    # --- snapshot 5: total=18 done=12 (last commit: today) ---
    cat > tasks.agile.md <<'EOF'
- [x] Stabilize parser edge-cases
- [x] Improve command help text
- [x] Refine ETA baseline
- [x] Harden config loading
- [x] Optimize task discovery
- [x] Polish history output
- [x] Tune velocity windowing
- [x] Add milestone summaries
- [x] Reduce startup overhead
- [x] Improve marker diagnostics
- [x] Refactor task traversal
- [x] Document CLI examples
- [ ] Improve cache invalidation notes
- [ ] Polish release checklist
- [ ] Generated sample task 15
- [ ] Generated sample task 16
- [ ] Generated sample task 17
- [ ] Generated sample task 18

#MILESTONE: Demo milestone

- [ ] Post-milestone follow-up

#MILESTONE: Second milestone

- [ ] Post-second-milestone follow-up
EOF
    commit_snapshot 5 "${dates[4]}"

    cd "$REPO_ROOT"
}

if [[ "${1:-}" != "--no-build" ]]; then
    build_agile_cli
fi

echo "Recreating sample repo: $TARGET_DIR (mode=new-milestone)"
create_sample_repo "$TARGET_DIR"
run_plot "$TARGET_DIR"
