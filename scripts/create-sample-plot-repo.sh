#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

declare -a MODES=("stable" "messy-total" "messy-done" "messy-both")

sample_dir_for_mode() {
    local mode="$1"
    case "$mode" in
        stable) echo "sample_dir_plot_stable" ;;
        messy-total) echo "sample_dir_plot_messy_total" ;;
        messy-done) echo "sample_dir_plot_messy_done" ;;
        messy-both) echo "sample_dir_plot_messy_both" ;;
        *)
            echo "error: unsupported mode: $mode" >&2
            exit 1
            ;;
    esac
}

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

    stable_totals=(12 12 12 12 13 13 13 14 14 14)
    stable_dones=(1 2 3 4 5 6 7 8 9 10)
    messy_total_totals=(12 14 11 15 13 16 14 17 15 18)
    messy_total_dones=(1 2 3 4 5 6 7 8 9 10)
    messy_done_totals=(13 13 13 13 14 14 14 14 15 15)
    messy_done_dones=(1 4 2 5 3 7 6 9 8 11)
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
        stable)
            totals=("${stable_totals[@]}")
            dones=("${stable_dones[@]}")
            ;;
        messy-total)
            totals=("${messy_total_totals[@]}")
            dones=("${messy_total_dones[@]}")
            ;;
        messy-done)
            totals=("${messy_done_totals[@]}")
            dones=("${messy_done_dones[@]}")
            ;;
        messy-both)
            totals=("${messy_both_totals[@]}")
            dones=("${messy_both_dones[@]}")
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

for mode in "${MODES[@]}"; do
    target_dir="$(sample_dir_for_mode "$mode")"
    echo "Recreating sample repo: $target_dir (mode=$mode)"
    create_sample_repo "$mode" "$target_dir"
    echo "Running plot command in $target_dir"
    devenv . --no-tty -a -c "cd $target_dir && ../target/debug/agile when --plot --next 1 >/dev/null"
done

echo "Created and validated sample repos:"
for mode in "${MODES[@]}"; do
    echo "  - $(sample_dir_for_mode "$mode")"
done
