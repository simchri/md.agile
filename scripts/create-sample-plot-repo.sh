#!/usr/bin/env bash
set -euo pipefail

MODE="stable"
TARGET_DIR="sample-plot-repo"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode)
            MODE="${2:-}"
            if [[ -z "$MODE" ]]; then
                echo "error: missing value for --mode" >&2
                exit 1
            fi
            shift 2
            ;;
        --help|-h)
            cat <<'EOF'
usage: ./scripts/create-sample-plot-repo.sh [target-dir] [--mode <mode>]

modes:
  stable
  messy-total
  messy-done
  messy-both
EOF
            exit 0
            ;;
        -*)
            echo "error: unknown option '$1' (expected --mode <stable|messy-total|messy-done|messy-both>)" >&2
            exit 1
            ;;
        *)
            if [[ "$TARGET_DIR" != "sample-plot-repo" ]]; then
                echo "error: multiple target directories specified ('$TARGET_DIR' and '$1')" >&2
                exit 1
            fi
            TARGET_DIR="$1"
            shift
            ;;
    esac
done

case "$MODE" in
    stable|messy-total|messy-done|messy-both) ;;
    *)
        echo "error: invalid mode '$MODE' (expected stable|messy-total|messy-done|messy-both)" >&2
        exit 1
        ;;
esac

if [[ -e "$TARGET_DIR" ]]; then
    echo "error: target already exists: $TARGET_DIR" >&2
    exit 1
fi

mkdir -p "$TARGET_DIR"
cd "$TARGET_DIR"

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
dates=(
    "2026-01-01T12:00:00Z"
    "2026-01-08T12:00:00Z"
    "2026-01-15T12:00:00Z"
    "2026-01-22T12:00:00Z"
    "2026-01-29T12:00:00Z"
    "2026-02-05T12:00:00Z"
    "2026-02-12T12:00:00Z"
    "2026-02-19T12:00:00Z"
    "2026-02-26T12:00:00Z"
    "2026-03-05T12:00:00Z"
)

totals=()
dones=()
case "$MODE" in
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

echo "created sample repo: $TARGET_DIR"
echo "commits: 10"
echo "mode: $MODE"
