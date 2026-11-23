#!/usr/bin/env bash
# In-game check for Building Gadgets 2 — the held build and the world-border edges.
#
# One launch: the carried gadget recipe is crafted in a real crafting table (with
# a missing-ingredient control), then one phase per mode the building gadget
# registers. Every phase clicks the south face of a stone block from inside the
# stage with the range set by `/buildinggadgets2 range` and the mode by
# `/buildinggadgets2 mode`, and asserts that the click:
#
#   * wrote **no** block (the world scan shows only the clicked block), because
#     this pin has no placement call a protection mod can cancel per position
#     (`runtime::placement_hold`, `blocked: no-cancellable-placement`);
#   * spent **no** item (the stone count is unchanged before and after);
#   * answered the parsed action bar with the exact number of cells the gate
#     would have allowed — the cell count the upstream mode produces, derived
#     from `util/modes/*.java`, not from the port's own planner.
#
# Two border phases then set a world border whose upper x edge falls exactly on a
# destination cell and one cell further out, and assert the count the gate
# reports: the cell on the upper bound is refused (the engine's upper edges are
# exclusive), the same cell one line inside is allowed.
#
#   GAME=<launcher-built horizon-client> tools/e2e.sh
#   GAME=$CONTROL_GAME tools/e2e.sh          # same Nightly, no package: the gadget
#                                            # id is unknown -> FAIL, EXIT:1
#   GAME=… SELFTEST=1 tools/e2e.sh           # every phase expects one cell placed
#                                            # and one cell too many in the reply,
#                                            # and must reject it -> FAIL, EXIT:1
#
# The SDK package is one flat `.hmod` that carries both halves, so the game must be
# built by the launcher with this package installed:
#
#   ports/tools/launcher-game.sh build buildinggadgets2
#   eval "$(ports/tools/launcher-game.sh env buildinggadgets2)"
#   ports/tools/build-slot.sh run --job W1-buildinggadgets2 --port ports/buildinggadgets2 \
#       --step e2e -- bash ports/buildinggadgets2/tools/e2e.sh
set -euo pipefail

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/buildinggadgets2
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
# `/tmp` is a 7.5 GB tmpfs: the launcher exports E2E_TMP on real disk, so honour it.
TMP=${E2E_TMP:-/tmp/opencode}
SOCK=${SOCK:-$TMP/buildinggadgets2-modes.sock}
SELFTEST=${SELFTEST:-0}
WORLD=${WORLD:-BuildingGadgets2Modes}
TOOL=buildinggadgets2:gadget_building
PHASES=11
HELD="Building Gadget: held (blocked: no-cancellable-placement)"

[ -n "$GAME" ] || {
    echo "GAME must name a launcher-built horizon-client carrying buildinggadgets2.hmod" >&2
    echo "  ports/tools/launcher-game.sh build buildinggadgets2" >&2
    exit 1
}
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

mkdir -p "$TMP" "$(dirname "$SOCK")"
SCRATCH=$(mktemp -d "$TMP/buildinggadgets2-home.XXXXXX")
GAME_DIR=$SCRATCH/game
export HORIZON_HOME=$GAME_DIR
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
rm -f "$SOCK"
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
count_stone() { ctl inventory | jq -r '[.counts["minecraft:stone"] // 0] | add'; }
sneaking() { ctl state | jq -r '.player.sneaking'; }
actionbar() { ctl messages 4 | jq -r '.actionbar // ""'; }

# The stone cells the gadget placed in a box, minus the clicked block, sorted.
placed_in() { # x1 y1 z1 x2 y2 z2 clicked-x clicked-y clicked-z
    ctl blocks "$1" "$2" "$3" "$4" "$5" "$6" \
        | jq -c --argjson cx "$7" --argjson cy "$8" --argjson cz "$9" \
            '[.blocks[] | select(.[3] == "minecraft:stone")
              | [.[0], .[1], .[2]]
              | select(. != [$cx, $cy, $cz])] | sort'
}
placed() { placed_in -8 147 -8 8 166 8 0 150 0; }

SELFTEST_FAILURES=0
# How many of the `PHASES` phases rejected the deliberately wrong expectation in a
# SELFTEST run: each phase counts once, however many of its assertions reject, so the
# final check proves every phase's assertion is fail-capable, not just one of them.
REJECTED_PHASES=0
PHASE_REJECTED=1
# A phase assertion. A SELFTEST run records the rejection and carries on, so one
# run proves every phase's assertion can fail; the normal run stops at the first.
phase_fail() {
    if [ "$SELFTEST" = 1 ]; then
        echo "SELFTEST: $*"
        SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))
        if [ "$PHASE_REJECTED" = 0 ]; then
            PHASE_REJECTED=1
            REJECTED_PHASES=$((REJECTED_PHASES + 1))
        fi
    else
        fail "$*"
    fi
}

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" || true
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() {
    rm -f "$SOCK"
    # `--offscreen`, not `--headless`: the recipe phase opens a crafting-table
    # menu and clicks its slots, and a headless client never opens a screen.
    "$CTL" --socket "$SOCK" launch --game "$GAME" --world "$WORLD" --seed 20261105 \
        --offscreen --wait --game-dir "$GAME_DIR" --resources "$RES"
}

# Clear every cell a phase may build in, keeping the plank platform at y=149
# (z=1..7) and re-placing the clicked stone block at (0,150,0).
reset_stage() {
    ctl fill -8 147 -8 8 166 0 minecraft:air
    ctl fill -8 150 1 8 166 8 minecraft:air
    ctl fill 0 150 0 0 150 0 minecraft:stone
    ctl tp 0 150 4
    ctl wait 1
    ctl look-at-block 0 150 0 south || fail "cannot aim at the clicked block's south face"
    ctl wait 1
}

select_block() {
    ctl key shift down
    ctl wait 1
    [ "$(sneaking)" = "true" ] || fail "the player is not sneaking after 'key shift down'"
    ctl use-item
    ctl wait 4
    ctl key shift up
    ctl wait 1
    [ "$(sneaking)" = "false" ] || fail "the player is still sneaking after 'key shift up'"
}

command_reply() { # the action bar the command just printed
    ctl chat "$1" >/dev/null
    ctl wait 2 >/dev/null
    actionbar
}

set_mode() { # name
    command_reply "/buildinggadgets2 mode $1"
}

set_place_on_top() { # on|off
    command_reply "/buildinggadgets2 placeontop $1"
}

# assert_held <name> <expected allowed cells> <placed cells json> <stone spent> <reply>
assert_held() {
    local name=$1 want=$2 actual=$3 spent=$4 reply=$5 want_reply want_placed cells
    want_placed='[]'
    if [ "$SELFTEST" = 1 ]; then
        # Invert the expectation: one more allowed cell and one placed block.
        want=$((want + 1))
        want_placed='[[99,99,99]]'
    fi
    cells=cells
    if [ "$want" -eq 1 ]; then cells=cell; fi
    want_reply="$HELD - $want $cells would be allowed, 0 placed, 0 spent"
    echo "  placed: $actual"
    echo "  consumed: $spent stone"
    echo "  reply: $reply"
    [ "$actual" = "$want_placed" ] \
        || phase_fail "$name: the click wrote $actual, expected $want_placed (the build is held: no cell may be written)"
    [ "$spent" = 0 ] \
        || phase_fail "$name: the click spent $spent stone on cells it did not fill, expected 0"
    [ "$reply" = "$want_reply" ] \
        || phase_fail "$name: the held reply was '$reply', expected '$want_reply'"
}

# phase <name> <mode|default> <range> <on|off> <upstream cells>
phase() {
    local name=$1 mode=$2 range=$3 on_top=$4 expected=$5
    local want before after spent actual reply

    echo "== phase $name: mode $mode, range $range, place-on-top $on_top =="
    PHASE_REJECTED=0
    reset_stage
    if [ "$mode" != "default" ]; then
        reply=$(set_mode "$mode")
        [ "$reply" = "Building Gadget mode set to $mode" ] \
            || phase_fail "$name: the mode reply was '$reply'"
    fi
    reply=$(command_reply "/buildinggadgets2 range $range")
    [ "$reply" = "Building Gadget range set to $range" ] \
        || phase_fail "$name: the range reply was '$reply'"
    if [ "$on_top" = "on" ]; then
        reply=$(set_place_on_top on)
        [ "$reply" = "Building Gadget place-on-top on" ] \
            || phase_fail "$name: the place-on-top reply was '$reply'"
    fi

    select_block
    ctl give minecraft:stone 40 >/dev/null
    ctl wait 2
    ctl select "$TOOL" >/dev/null

    want=$(printf '%s' "$expected" | jq 'length')
    before=$(count_stone)
    ctl use-item >/dev/null
    ctl wait 4
    reply=$(actionbar)
    after=$(count_stone)
    spent=$((before - after))
    actual=$(placed)
    assert_held "$name" "$want" "$actual" "$spent" "$reply"
}

# border_phase <name> <worldborder size> <expected allowed cells>
# The wall is `vertical_wall` around the clicked block at (0,150,0) with range 5, so
# its cells are x -2..2, y 148..152 at z=0 (the clicked cell is solid and skipped).
# A border centred at (0,0) of size 4 ends at x=2: the cells at -2..1 pass and the
# column at exactly 2 is out (4 columns x 5 rows, minus the clicked block = 19).
# Size 6 ends at x=3, so the column at 2 is one cell inside and passes (5 x 5 - 1 = 24).
border_phase() {
    local name=$1 size=$2 want=$3
    local before after spent actual reply edge

    edge=$((size / 2))
    echo "== border phase $name: worldborder size $size, upper x edge at $edge =="
    PHASE_REJECTED=0
    reset_stage
    ctl command "worldborder center 0 0" >/dev/null
    ctl command "worldborder set $size" >/dev/null
    ctl wait 2
    reply=$(ctl command "worldborder get" | jq -r '.chat[0] // ""')
    echo "  worldborder get: $reply"
    # The gate is per destination, so the border cuts the wall's columns wherever the
    # builder stands; the player's own position is printed so the log says where.
    echo "  player: $(ctl state | jq -c '.player.position')"

    reply=$(set_mode vertical_wall)
    [ "$reply" = "Building Gadget mode set to vertical_wall" ] \
        || phase_fail "$name: the mode reply was '$reply'"
    reply=$(command_reply "/buildinggadgets2 range 5")
    [ "$reply" = "Building Gadget range set to 5" ] \
        || phase_fail "$name: the range reply was '$reply'"

    select_block
    ctl give minecraft:stone 40 >/dev/null
    ctl wait 2
    ctl select "$TOOL" >/dev/null

    before=$(count_stone)
    ctl use-item >/dev/null
    ctl wait 4
    reply=$(actionbar)
    after=$(count_stone)
    spent=$((before - after))
    actual=$(placed)
    assert_held "$name" "$want" "$actual" "$spent" "$reply"
}

echo "== launch (survival world $WORLD; the package is installed in the instance) =="
launch "$WORLD"
ctl wait-ready 120

echo "== a plank platform under the player at y=149; the clicked block floats at (0,150,0) =="
ctl command "time set noon"
ctl command "difficulty peaceful"
ctl command "worldborder set 59999968"
ctl command "kill @e[type=!minecraft:player,type=!minecraft:item]"
ctl command "clear @s"
ctl fill -8 140 -8 8 166 8 minecraft:air
ctl fill -4 149 1 4 149 7 minecraft:oak_planks
ctl fill 0 150 0 0 150 0 minecraft:stone
ctl tp 0 150 4
ctl wait 2

echo "== the carried gadget recipe crafts in a real crafting table =="
ctl setblock 3 150 4 minecraft:crafting_table
ctl wait 2
python3 "$PORT/tools/craft_gadget.py" "$SOCK" "$CTL" 3 150 4 \
    || fail "the carried gadget recipe did not craft (or its control matched)"
ctl command "clear @s"
ctl wait 2
ctl tp 0 150 4
ctl wait 2

ctl give "$TOOL" 1 >/dev/null || fail "the package did not provide $TOOL"
ctl wait 2
[ "$(ctl inventory | jq -r '[.counts["'"$TOOL"'"] // 0] | add')" -ge 1 ] \
    || fail "$TOOL is missing from the inventory"
ctl select "$TOOL" >/dev/null || fail "cannot select $TOOL"
ctl wait 2
held=$(ctl inventory | jq -r '.held.item // "none"')
echo "held: $held"
[ "$held" = "$TOOL" ] || fail "the held item is $held, expected $TOOL"

echo "== the status reply says the build is held and why (mode is still the fresh default) =="
reply=$(command_reply "/buildinggadgets2 range 5")
[ "$reply" = "Building Gadget range set to 5" ] || fail "the range reply was '$reply'"
reply=$(command_reply "/buildinggadgets2")
echo "  reply: $reply"
[ "$reply" = "Building Gadget: mode build_to_me, range 5, block none, place-on-top off, placement held (blocked: no-cancellable-placement)" ] \
    || phase_fail "the status reply was '$reply'"

# The default mode is a fresh gadget's `build_to_me`: no mode command has run yet.
phase build_to_me default 5 off '[[0,150,1],[0,150,2],[0,150,3]]'
phase horizontal_row horizontal_row 5 off '[[0,150,-1],[0,150,-2],[0,150,-3],[0,150,-4]]'
phase vertical_column vertical_column 5 off '[[0,148,0],[0,149,0],[0,151,0],[0,152,0]]'
phase vertical_wall vertical_wall 5 off "$(jq -cn '[range(-2;3) as $x | range(148;153) as $y | select($x != 0 or $y != 150) | [$x,$y,0]]')"
phase horizontal_wall horizontal_wall 3 off "$(jq -cn '[range(-1;2) as $x | range(1;4) as $z | [$x,150,$z]]')"
phase surface surface 5 off '[[0,150,1]]'
phase stairs stairs 5 off '[[0,151,-1],[0,152,-2],[0,153,-3],[0,154,-4]]'
phase grid grid 5 off "$(jq -cn '[range(-6;7;6) as $x | range(-6;7;6) as $z | select($x != 0 or $z != 0) | [$x,150,$z]]')"
phase grid_on_top grid 5 on "$(jq -cn '[range(-6;7;6) as $x | range(-6;7;6) as $z | [$x,151,$z]]')"

echo "== the status reply names the grid mode, the place-on-top setting and the hold =="
reply=$(command_reply "/buildinggadgets2")
echo "  reply: $reply"
[ "$reply" = "Building Gadget: mode grid, range 5, block minecraft:stone, place-on-top on, placement held (blocked: no-cancellable-placement)" ] \
    || phase_fail "the final status reply was '$reply'"

# The world border's upper edges are exclusive: with the edge at x=19 the column at
# exactly 19 is refused (3 columns x 5 rows, minus the clicked block = 14), and with
# the edge at x=20 the same column is one cell inside and allowed (4 x 5 - 1 = 19).
border_phase edge_at_the_upper_bound 4 19
border_phase one_cell_inside_the_bound 6 24
ctl command "worldborder set 59999968" >/dev/null

if [ "$SELFTEST" = 1 ]; then
    [ "$REJECTED_PHASES" -ge "$PHASES" ] \
        || fail "SELFTEST: only $REJECTED_PHASES of $PHASES phases rejected the wrong expectation ($SELFTEST_FAILURES assertions)"
    echo "FAIL: SELFTEST: all $PHASES phases rejected a deliberately wrong expectation ($SELFTEST_FAILURES assertions)"
    exit 1
fi

echo "PASS: the carried recipe crafted one gadget with its ingredients consumed (a missing lapis matched nothing); all 9 modes held the build with the exact upstream cell count, 0 blocks written and 0 stone spent; and the world border's upper edge refused the cell exactly on it while allowing it one cell inside"
