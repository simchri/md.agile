#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/plot-demo-common.bash
source "$SCRIPT_DIR/lib/plot-demo-common.bash"

TARGET_DIR="sample_dir_plot_second_milestone"

create_sample_repo() {
    local target_dir="$1"

    cd "$REPO_ROOT"
    init_sample_repo "$target_dir"

    compute_dates 5 14

    # --- snapshot 1: total=16 done=3. Task 6 (before the "Demo milestone"
    # marker) is left undone in every snapshot so that milestone never
    # becomes fully reached, keeping "Demo milestone 2" at rank 2 throughout ---
    cat > tasks.agile.md <<'EOF'
- [x] Task 1
- [x] Task 2
- [x] Task 3
- [ ] Task 4
- [ ] Task 5
- [ ] Task 6

#MILESTONE: Demo milestone 

- [ ] Task 7
- [ ] Task 8
- [ ] Task 9
- [ ] Task 10
- [ ] Task 11
- [ ] Task 12
- [ ] Task 13
- [ ] Task 14
- [ ] Task 15

#MILESTONE: Demo milestone 2

- [ ] Task 16

other notes 1
EOF
    commit_snapshot 1 "${dates[0]}"

    # --- snapshot 2: total=16 done=6. Task 6 stays undone (see snapshot 1) so
    # "Demo milestone" never becomes fully reached; progress toward it instead
    # comes from completing tasks 7-15, keeping "Demo milestone 2" at rank 2 ---
    cat > tasks.agile.md <<'EOF'
- [x] Task 1
- [x] Task 2
- [x] Task 3
- [x] Task 4
- [ ] Task 5
- [ ] Task 6

#MILESTONE: Demo milestone 

- [x] Task 7
- [x] Task 8
- [ ] Task 9
- [ ] Task 10
- [ ] Task 11
- [ ] Task 12
- [ ] Task 13
- [ ] Task 14
- [ ] Task 15

#MILESTONE: Demo milestone 2

- [ ] Task 16

other notes 2
EOF
    commit_snapshot 2 "${dates[1]}"

    # --- snapshot 3: total=16 done=8 (Task 6 still undone) ---
    cat > tasks.agile.md <<'EOF'
- [x] Task 1
- [x] Task 2
- [x] Task 3
- [x] Task 4
- [x] Task 5
- [ ] Task 6

#MILESTONE: Demo milestone 

- [x] Task 7
- [x] Task 8
- [x] Task 9
- [ ] Task 10
- [ ] Task 11
- [ ] Task 12
- [ ] Task 13
- [ ] Task 14
- [ ] Task 15

#MILESTONE: Demo milestone 2

- [ ] Task 16

other notes 3
EOF
    commit_snapshot 3 "${dates[2]}"

    # --- snapshot 4: total=16 done=10 (Task 6 still undone) ---
    cat > tasks.agile.md <<'EOF'
- [x] Task 1
- [x] Task 2
- [x] Task 3
- [x] Task 4
- [x] Task 5
- [ ] Task 6

#MILESTONE: Demo milestone 

- [x] Task 7
- [x] Task 8
- [x] Task 9
- [x] Task 10
- [x] Task 11
- [ ] Task 12
- [ ] Task 13
- [ ] Task 14
- [ ] Task 15

#MILESTONE: Demo milestone 2

- [ ] Task 16

other notes 4
EOF
    commit_snapshot 4 "${dates[3]}"

    # --- snapshot 5: total=16 done=12 (last commit: today). Task 6 is still
    # undone here, so "Demo milestone" remains a future milestone and
    # "Demo milestone 2" stays rank 2, making `--next 2` resolve correctly ---
    cat > tasks.agile.md <<'EOF'
- [x] Task 1
- [x] Task 2
- [x] Task 3
- [x] Task 4
- [x] Task 5
- [ ] Task 6

#MILESTONE: Demo milestone 

- [x] Task 7
- [x] Task 8
- [x] Task 9
- [x] Task 10
- [x] Task 11
- [x] Task 12
- [x] Task 13
- [ ] Task 14
- [ ] Task 15

#MILESTONE: Demo milestone 2

- [ ] Task 16

other notes 5
EOF
    commit_snapshot 5 "${dates[4]}"

    cd "$REPO_ROOT"
}

if [[ "${1:-}" != "--no-build" ]]; then
    build_agile_cli
fi

echo "Recreating sample repo: $TARGET_DIR (mode=new-milestone)"
create_sample_repo "$TARGET_DIR"

run_plot_cust() {
    local target_dir="$1"
    echo "Running plot command in $target_dir"
    echo "----- plot output: $target_dir -----"
    CLICOLOR_FORCE=1 devenv "${PLOT_DEVENV_ARGS[@]}" -c "cd $target_dir && ../target/debug/agile when --plot --next 2"
    echo "----- end plot output: $target_dir -----"
    echo "Created and validated sample repo: $target_dir"
}

run_plot_cust "$TARGET_DIR"
