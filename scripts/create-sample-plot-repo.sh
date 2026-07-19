#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

MODE="messy-both"
TARGET_DIR="sample_dir_plot_messy_both"
declare -a PLOT_DEVENV_ARGS=(. -a)
if [[ ! -t 0 || ! -t 1 ]]; then
    PLOT_DEVENV_ARGS=(. --no-tty -a)
fi

create_sample_repo() {
    local mode="$1"
    local target_dir="$2"

    rm -rf "$target_dir"
    mkdir -p "$target_dir"
    cd "$target_dir"

    git init -q
    git config user.name "Plot Sample"
    git config user.email "plot-sample@example.com"

    cat > mdagile.toml <<'EOF'
[General]
warn_when_not_a_git_repo = true
EOF

    # Commits a snapshot: dumps the given tasks.agile.md content to disk
    # verbatim, then creates a commit timed at the given date.
    commit_snapshot() {
        local index="$1"
        local date="$2"
        git add -A
        GIT_AUTHOR_DATE="$date" GIT_COMMITTER_DATE="$date" \
            git commit -q -m "snapshot $index"
    }

    dates=()
    for i in $(seq 0 9); do
        days_ago=$(((9 - i) * 7))
        iso_date="$(date -u -d "$days_ago days ago" +%Y-%m-%d)T12:00:00Z"
        dates+=("$iso_date")
    done

    case "$mode" in
        messy-both) ;;
        *)
            echo "error: unsupported mode: $mode" >&2
            exit 1
            ;;
    esac

    # --- snapshot 1: total=12 done=1 ---
    cat > tasks.agile.md <<'EOF'
- [x] Stabilize parser edge-cases
- [ ] Improve command help text
- [ ] Refine ETA baseline
- [ ] Harden config loading
- [ ] Optimize task discovery
- [ ] Polish history output
- [ ] Tune velocity windowing
- [ ] Add milestone summaries
- [ ] Reduce startup overhead
- [ ] Improve marker diagnostics
- [ ] Refactor task traversal
- [ ] Document CLI examples

#MILESTONE: Demo milestone

- [ ] Post-milestone follow-up
EOF
    commit_snapshot 1 "${dates[0]}"

    # --- snapshot 2: total=15 done=3 ---
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
    commit_snapshot 2 "${dates[1]}"

    # --- snapshot 3: total=11 done=2 ---
    cat > tasks.agile.md <<'EOF'
- [x] Stabilize parser edge-cases
- [x] Improve command help text
- [ ] Refine ETA baseline
- [ ] Harden config loading
- [ ] Optimize task discovery
- [ ] Polish history output
- [ ] Tune velocity windowing
- [ ] Add milestone summaries
- [ ] Reduce startup overhead
- [ ] Improve marker diagnostics
- [ ] Refactor task traversal

#MILESTONE: Demo milestone

- [ ] Post-milestone follow-up
EOF
    commit_snapshot 3 "${dates[2]}"

    # --- snapshot 4: total=14 done=6 ---
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
    commit_snapshot 4 "${dates[3]}"

    # --- snapshot 5: total=13 done=4 ---
    cat > tasks.agile.md <<'EOF'
- [x] Stabilize parser edge-cases
- [x] Improve command help text
- [x] Refine ETA baseline
- [x] Harden config loading
- [ ] Optimize task discovery
- [ ] Polish history output
- [ ] Tune velocity windowing
- [ ] Add milestone summaries
- [ ] Reduce startup overhead
- [ ] Improve marker diagnostics
- [ ] Refactor task traversal
- [ ] Document CLI examples
- [ ] Improve cache invalidation notes

#MILESTONE: Demo milestone

- [ ] Post-milestone follow-up
EOF
    commit_snapshot 5 "${dates[4]}"

    # --- snapshot 6: total=16 done=8 ---
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
    commit_snapshot 6 "${dates[5]}"

    # --- snapshot 7: total=14 done=7 ---
    cat > tasks.agile.md <<'EOF'
- [x] Stabilize parser edge-cases
- [x] Improve command help text
- [x] Refine ETA baseline
- [x] Harden config loading
- [x] Optimize task discovery
- [x] Polish history output
- [x] Tune velocity windowing
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
    commit_snapshot 7 "${dates[6]}"

    # --- snapshot 8: total=17 done=10 ---
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
EOF
    commit_snapshot 8 "${dates[7]}"

    # --- snapshot 9: total=15 done=9 ---
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
- [ ] Improve marker diagnostics
- [ ] Refactor task traversal
- [ ] Document CLI examples
- [ ] Improve cache invalidation notes
- [ ] Polish release checklist
- [ ] Generated sample task 15

#MILESTONE: Demo milestone

- [ ] Post-milestone follow-up
EOF
    commit_snapshot 9 "${dates[8]}"

    # --- snapshot 10: total=18 done=12 (last commit: today) ---
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
EOF
    commit_snapshot 10 "${dates[9]}"

    cd "$REPO_ROOT"
}

echo "Building agile CLI..."
devenv . --no-tty -a -c "cargo build --bin agile"

echo "Recreating sample repo: $TARGET_DIR (mode=$MODE)"
create_sample_repo "$MODE" "$TARGET_DIR"
echo "Running plot command in $TARGET_DIR"
echo "----- plot output: $TARGET_DIR -----"
CLICOLOR_FORCE=1 devenv "${PLOT_DEVENV_ARGS[@]}" -c "cd $TARGET_DIR && ../target/debug/agile when --plot --next 1"
echo "----- end plot output: $TARGET_DIR -----"
echo "Created and validated sample repo: $TARGET_DIR"
