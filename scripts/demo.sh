#!/usr/bin/env bash
# Demo script: drives tasks across the md.agile GUI board.
#
# Usage:
#   ./scripts/demo.sh [mode]
#
# Modes:
#   overtake  (default) — few cards, shows collision and overtake
#   many                — 40 static in-progress cards + 10 traversers that
#                         move through the crowded board from backlog to done
#
# Optional env vars:
#   MDAGILE_DEMO_DELAY   seconds per phase (default: 4)
set -euo pipefail

MODE="${1:-overtake}"
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

echo "[demo] Mode: $MODE | Fixture: $FIXTURE_DIR"
touch "$FIXTURE_DIR/mdagile.toml"

# ---------------------------------------------------------------------------
# Shared helper
# ---------------------------------------------------------------------------

# task <parent_done 0|1> <title> <body> <total_subtasks> <done_subtasks>
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

# ---------------------------------------------------------------------------
# Overtake mode
# ---------------------------------------------------------------------------

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

run_overtake() {
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

    local LOOP=0
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
}

# ---------------------------------------------------------------------------
# Many-cards mode
# ---------------------------------------------------------------------------

# write_many_phase <traveler_done>
#   traveler_done: integer 0..10 (subtasks done out of 10), or "x" (parent done).
#
# Writes:
#   30 background in-progress cards — clustered mostly at 50%, with smaller
#     groups at 10%, 20%, 60%, 80% (subtasks done out of 9):
#   1 Traveler card — starts in backlog, then marches through the board
#     (4 subtasks; 1 subtask per phase → progress 0.225 per phase step)
write_many_phase() {
    local tdone="$1"
    {
        # 30 cards: 4 at 0.10, 4 at 0.20, 14 at 0.50, 4 at 0.60, 4 at 0.80
        # (level = subtasks done out of 9; progress = level/9 * 0.9)
        local LEVELS="1 1 1 1  2 2 2 2  5 5 5 5 5 5 5 5 5 5 5 5 5 5  6 6 6 6  8 8 8 8"
        local n=1
        local level
        for level in $LEVELS; do
            local body
            case "$((n % 5))" in
                1) body="Lorem ipsum dolor sit amet, consectetur adipiscing elit." ;;
                2) body="Ut enim ad minim veniam, quis nostrud exercitation laboris." ;;
                3) body="Duis aute irure dolor in reprehenderit in voluptate esse." ;;
                4) body="Excepteur sint occaecat cupidatat non proident culpa." ;;
                0) body="Nemo enim ipsam voluptatem quia voluptas sit aspernatur." ;;
            esac
            printf -- "- [ ] Card%s\n  %s\n" "$n" "$body"
            local s
            for s in $(seq 1 9); do
                if [[ "$s" -le "$level" ]]; then
                    printf "  - [x] Card%s step %s\n" "$n" "$s"
                else
                    printf "  - [ ] Card%s step %s\n" "$n" "$s"
                fi
            done
            printf "\n"
            n=$((n + 1))
        done

        if [[ "$tdone" == "x" ]]; then
            printf -- "- [x] Traveler\n  Traveler crossing the crowded board.\n"
            local s
            for s in $(seq 1 4); do
                printf "  - [x] Traveler step %s\n" "$s"
            done
        else
            printf -- "- [ ] Traveler\n  Traveler crossing the crowded board.\n"
            local s
            for s in $(seq 1 4); do
                if [[ "$s" -le "$tdone" ]]; then
                    printf "  - [x] Traveler step %s\n" "$s"
                else
                    printf "  - [ ] Traveler step %s\n" "$s"
                fi
            done
        fi
        printf "\n"
    } > "$TASKS_FILE"
}

run_many() {
    # Progress legend:
    #   Background cards: 30 static cards — 4 at 0.10, 4 at 0.20, 14 at 0.50,
    #                     4 at 0.60, 4 at 0.80
    #   Traveler (1 card, 4 subtasks):
    #     Phase 1: 0/4 done → backlog
    #     Phase 2: 1/4 done → progress 0.225
    #     Phase 3: 2/4 done → progress 0.45
    #     Phase 4: 3/4 done → progress 0.675
    #     Phase 5: 4/4 done → progress 0.90
    #     Phase 6: parent [x] → done strip
    local LOOP=0
    while true; do
        LOOP=$((LOOP + 1))
        printf "[demo/many] ─── Loop %-3d ─────────────────────────────────────\n" "$LOOP"

        printf "[demo/many]  1/6  30 cards in progress; traveler in backlog\n"
        write_many_phase 0
        sleep "$STEP_SECS"

        local step
        for step in $(seq 1 4); do
            local phase=$((step + 1))
            printf "[demo/many]  %d/6  Travelers: %d/4 subtasks done\n" "$phase" "$step"
            write_many_phase "$step"
            sleep "$STEP_SECS"
        done

        printf "[demo/many]  6/6  Travelers done — resetting in 3s\n"
        write_many_phase "x"
        sleep "$STEP_SECS"
        sleep 3
        echo ""
    done
}

# ---------------------------------------------------------------------------
# Server startup
# ---------------------------------------------------------------------------

echo "[demo] Building and starting dx serve (log: $DX_LOG)"
echo "[demo] First build may take ~60s..."
(cd "$GUI_DIR" && MDAGILE_WORKDIR="$FIXTURE_DIR" dx serve --hot-patch --hot-reload=true >"$DX_LOG" 2>&1) &
DX_PID=$!

echo "[demo] Waiting for server on :8080 ..."
WAIT=0
set +e
until curl -sf http://localhost:8080 >/dev/null 2>&1; do
    # Check if dx process is still running
    if ! kill -0 "$DX_PID" 2>/dev/null; then
        echo ""
        echo "[demo] ERROR: dx process crashed or failed to start. Check $DX_LOG for errors."
        cat "$DX_LOG"
        exit 1
    fi
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
echo "[demo] Starting '$MODE' loop (${STEP_SECS}s per phase, Ctrl-C to stop)"
echo ""

# ---------------------------------------------------------------------------
# Dispatch
# ---------------------------------------------------------------------------

case "$MODE" in
    overtake) run_overtake ;;
    many)     run_many ;;
    *)
        echo "[demo] Unknown mode: $MODE"
        echo "[demo] Usage: $0 [overtake|many]"
        exit 1
        ;;
esac
