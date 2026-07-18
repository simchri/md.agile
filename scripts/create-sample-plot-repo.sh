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

    task_name() {
        local idx="$1"
        local names=(
            "Stabilize parser edge-cases"
            "Improve command help text"
            "Refine ETA baseline"
            "Harden config loading"
            "Optimize task discovery"
            "Polish history output"
            "Tune velocity windowing"
            "Add milestone summaries"
            "Reduce startup overhead"
            "Improve marker diagnostics"
            "Refactor task traversal"
            "Document CLI examples"
            "Improve cache invalidation notes"
            "Polish release checklist"
        )
        local array_idx=$((idx - 1))
        if [[ "$array_idx" -lt "${#names[@]}" ]]; then
            echo "${names[$array_idx]}"
        else
            echo "Generated sample task $idx"
        fi
    }

    write_snapshot() {
        local total="$1"
        local done="$2"
        : > tasks.agile.md
        for i in $(seq 1 "$total"); do
            local box="[ ]"
            if [[ "$i" -le "$done" ]]; then
                box="[x]"
            fi
            printf -- "- %s %s\n" "$box" "$(task_name "$i")" >> tasks.agile.md
        done
        printf "\n#MILESTONE: Demo milestone\n\n" >> tasks.agile.md
        printf -- "- [ ] Post-milestone follow-up\n" >> tasks.agile.md
    }

    commit_snapshot() {
        local index="$1"
        local total="$2"
        local done="$3"
        local date="$4"
        write_snapshot "$total" "$done"
        git add -A
        GIT_AUTHOR_DATE="$date" GIT_COMMITTER_DATE="$date" \
            git commit -q -m "snapshot $index: total=$total done=$done"
    }

    messy_both_totals=(12 15 11 14 13 16 14 17 15 18)
    messy_both_dones=(1 3 2 6 4 8 7 10 9 12)
    dates=()
    for i in $(seq 0 9); do
        days_ago=$(((9 - i) * 7))
        iso_date="$(date -u -d "$days_ago days ago" +%Y-%m-%d)T12:00:00Z"
        dates+=("$iso_date")
    done

    totals=()
    dones=()
    case "$mode" in
        messy-both)
            totals=("${messy_both_totals[@]}")
            dones=("${messy_both_dones[@]}")
            ;;
        *)
            echo "error: unsupported mode: $mode" >&2
            exit 1
            ;;
    esac

    for i in "${!totals[@]}"; do
        step=$((i + 1))
        commit_snapshot "$step" "${totals[$i]}" "${dones[$i]}" "${dates[$i]}"
    done

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
