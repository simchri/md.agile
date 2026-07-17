#!/usr/bin/env bash
set -euo pipefail

TARGET_DIR="${1:-sample-plot-repo}"
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
    echo "${names[$((idx - 1))]}"
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

totals=(12 12 12 12 13 13 13 14 14 14)
dones=(1 2 3 4 5 6 7 8 9 10)
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

for i in "${!totals[@]}"; do
    step=$((i + 1))
    commit_snapshot "$step" "${totals[$i]}" "${dones[$i]}" "${dates[$i]}"
done

echo "created sample repo: $TARGET_DIR"
echo "commits: 10"
