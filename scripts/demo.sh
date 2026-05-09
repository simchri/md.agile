#!/usr/bin/env bash
# Demo script: drives tasks across the md.agile GUI board to demonstrate
# the repel/spread feature need.
#
# Scenario highlights:
#   - Multiple tasks in progress simultaneously
#   - Phase 5: Gamma and Delta converge to the same position (0.45) — COLLISION
#   - Phase 6: Delta overtakes Gamma — OVERTAKE
#
# Usage:
#   ./scripts/demo.sh
#
# Optional env vars:
#   MDAGILE_DEMO_DELAY   seconds per phase (default: 4)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GUI_DIR="$REPO_ROOT/crates/gui"
FIXTURE_DIR="$(mktemp -d /tmp/mdagile-demo-XXXXXX)"
TASKS_FILE="$FIXTURE_DIR/demo.agile.md"
STEP_SECS="${MDAGILE_DEMO_DELAY:-4}"
DX_LOG="$FIXTURE_DIR/dx.log"

cleanup() {
    echo ""
    echo "[demo] Stopping."
    kill $(jobs -p) 2>/dev/null || true
    rm -rf "$FIXTURE_DIR"
}
trap cleanup EXIT INT TERM

echo "[demo] Fixture: $FIXTURE_DIR"
touch "$FIXTURE_DIR/mdagile.toml"

# task <parent_done 0|1> <title> <body> <total_subtasks> <done_subtasks>
# Writes one task block to stdout.
task() {
    local pd="$1" title="$2" body="$3" total="$4" done="$5"
    local ps="[ ]"
    [[ "$pd" == "1" ]] && ps="[x]"
    printf -- "- %s %s\n  %s\n" "$ps" "$title" "$body"
    local i
    for i in $(seq 1 "$total"); do
        local ss="[ ]"
        [[ "$i" -le "$done" ]] && ss="[x]"
        printf "  - %s %s subtask %s\n" "$ss" "$title" "$i"
    done
    printf "\n"
}

# write_phase <a_pd> <a_d>  <b_pd> <b_d>  <g_pd> <g_d>  <d_pd> <d_d> <d_show>  <e_pd> <e_d> <e_show>
#
# Task legend:
#   Alpha   3 subtasks  fast mover
#   Beta    5 subtasks  medium mover
#   Gamma   6 subtasks  slow mover (gets overtaken by Delta)
#   Delta   4 subtasks  fast mover (overtakes Gamma)
#   Epsilon 2 subtasks  very fast (appears late)
#   Zeta    2 subtasks  always backlog
#   Eta     3 subtasks  always backlog
write_phase() {
    {
        task "$1"    "Alpha"   "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod." 3 "$2"
        task "$3"    "Beta"    "Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris." 5 "$4"
        task "$5"    "Gamma"   "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum." 6 "$6"
        [[ "${9}"   == "1" ]] && task "$7"    "Delta"   "Excepteur sint occaecat cupidatat non proident in culpa." 4 "$8"
        [[ "${12}"  == "1" ]] && task "${10}" "Epsilon" "Nemo enim ipsam voluptatem quia voluptas sit aspernatur." 2 "${11}"
        task "0" "Zeta" "At vero eos et accusamus et iusto odio dignissimos ducimus blanditiis." 2 0
        task "0" "Eta"  "Nam libero tempore cum soluta nobis eligendi optio cumque nihil impedit." 3 0
    } > "$TASKS_FILE"
}

# Start GUI server
echo "[demo] Building and starting dx serve (log: $DX_LOG)"
echo "[demo] First build may take ~60s..."
(cd "$GUI_DIR" && MDAGILE_WORKDIR="$FIXTURE_DIR" dx serve >"$DX_LOG" 2>&1) &

echo "[demo] Waiting for server on :8080 ..."
WAIT=0
set +e
until curl -sf http://localhost:8080 >/dev/null 2>&1; do
    sleep 2
    WAIT=$((WAIT + 2))
    printf "[demo]   ...%ds elapsed\r" "$WAIT"
    if [[ "$WAIT" -ge 180 ]]; then
        echo ""
        echo "[demo] Timeout waiting for server. Check $DX_LOG for errors."
        exit 1
    fi
done
set -e

echo ""
echo "[demo] Server ready — open http://localhost:8080"
echo "[demo] Starting demo loop (${STEP_SECS}s per phase, Ctrl-C to stop)"
echo ""

# Progress legend for each phase (progress = subtasks_done/total * 0.9):
#
#  Phase  Alpha  Beta   Gamma  Delta  Epsilon  Notes
#    1    0.00   0.00   0.00   ---    ---      All backlog
#    2    0.30   0.18   0.00   ---    ---      Alpha+Beta on diagonal
#    3    0.60   0.36   0.15   0.00   ---      Gamma joins; Delta enters backlog
#    4    DONE   0.54   0.30   0.225  ---      Alpha done; Delta starts (behind Gamma)
#    5    DONE   0.72   0.45   0.45   0.00     COLLISION: Gamma=Delta=0.45; Epsilon backlog
#    6    DONE   DONE   0.60   0.675  0.45     OVERTAKE: Delta(0.675) passes Gamma(0.60)
#    7    DONE   DONE   0.75   DONE   DONE     Delta+Epsilon done; Gamma moves on
#    8    DONE   DONE   0.90   DONE   DONE     Gamma: all subtasks done, parent unticked
#    9    DONE   DONE   DONE   DONE   DONE     All complete — loop resets

LOOP=0
while true; do
    LOOP=$((LOOP + 1))
    printf "[demo] ─── Loop %-3d ──────────────────────────────────────────\n" "$LOOP"

    printf "[demo]  1/9  All tasks in backlog\n"
    write_phase  0 0  0 0  0 0  0 0 0  0 0 0
    sleep "$STEP_SECS"

    printf "[demo]  2/9  Alpha(0.30) + Beta(0.18) enter diagonal\n"
    write_phase  0 1  0 1  0 0  0 0 0  0 0 0
    sleep "$STEP_SECS"

    printf "[demo]  3/9  Gamma(0.15) joins; Delta enters backlog\n"
    write_phase  0 2  0 2  0 1  0 0 1  0 0 0
    sleep "$STEP_SECS"

    printf "[demo]  4/9  Alpha done; Delta(0.225) starts — behind Gamma(0.30)\n"
    write_phase  1 3  0 3  0 2  0 1 1  0 0 0
    sleep "$STEP_SECS"

    printf "[demo]  5/9  *** COLLISION: Gamma=0.45 meets Delta=0.45; Epsilon enters backlog\n"
    write_phase  1 3  0 4  0 3  0 2 1  0 0 1
    sleep "$STEP_SECS"

    printf "[demo]  6/9  *** OVERTAKE: Delta(0.675) passes Gamma(0.60); Epsilon(0.45) on board\n"
    write_phase  1 3  1 5  0 4  0 3 1  0 1 1
    sleep "$STEP_SECS"

    printf "[demo]  7/9  Delta + Epsilon done; Gamma(0.75) still moving\n"
    write_phase  1 3  1 5  0 5  1 4 1  1 2 1
    sleep "$STEP_SECS"

    printf "[demo]  8/9  Gamma(0.90) — all subtasks done, parent not yet ticked\n"
    write_phase  1 3  1 5  0 6  1 4 1  1 2 1
    sleep "$STEP_SECS"

    printf "[demo]  9/9  All done — resetting in 3s\n"
    write_phase  1 3  1 5  1 6  1 4 1  1 2 1
    sleep 3
    echo ""
done
