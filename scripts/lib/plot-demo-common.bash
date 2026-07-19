# Shared helpers for scripts/plot-demo-*.bash sub-scripts.
# Source this file after setting SCRIPT_DIR and REPO_ROOT.

declare -a PLOT_DEVENV_ARGS=(. -a)
if [[ ! -t 0 || ! -t 1 ]]; then
    PLOT_DEVENV_ARGS=(. --no-tty -a)
fi

# Initializes a fresh git repo (with mdagile.toml) at $1, and cd's into it.
init_sample_repo() {
    local target_dir="$1"

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
}

# Commits a snapshot: dumps the already-written tasks.agile.md content to
# disk verbatim (via `git add`), then creates a commit timed at the given
# date. Call after writing tasks.agile.md for the snapshot.
commit_snapshot() {
    local index="$1"
    local date="$2"
    git add -A
    GIT_AUTHOR_DATE="$date" GIT_COMMITTER_DATE="$date" \
        git commit -q -m "snapshot $index"
}

# Fills the global "dates" array with $1 ISO dates, spaced $2 days apart,
# ending today (dates[$1-1] is today).
compute_dates() {
    local count="$1"
    local spacing_days="$2"
    dates=()
    for i in $(seq 0 $((count - 1))); do
        local days_ago=$(( (count - 1 - i) * spacing_days ))
        local iso_date
        iso_date="$(date -u -d "$days_ago days ago" +%Y-%m-%d)T12:00:00Z"
        dates+=("$iso_date")
    done
}

# Builds the agile CLI (from repo root).
build_agile_cli() {
    echo "Building agile CLI..."
    (cd "$REPO_ROOT" && devenv . --no-tty -a -c "cargo build --bin agile")
}

# Runs `agile when --plot` in the given target dir (relative to repo root)
# and prints its output between markers.
run_plot() {
    local target_dir="$1"
    echo "Running plot command in $target_dir"
    echo "----- plot output: $target_dir -----"
    CLICOLOR_FORCE=1 devenv "${PLOT_DEVENV_ARGS[@]}" -c "cd $target_dir && ../target/debug/agile when --plot --next 1"
    echo "----- end plot output: $target_dir -----"
    echo "Created and validated sample repo: $target_dir"
}
