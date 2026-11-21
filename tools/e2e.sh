#!/usr/bin/env bash
# In-game check for Building Gadgets 2 — the mode family.
#
# One launch: the carried gadget recipe is crafted in a real crafting table
# (with a missing-ingredient control), then one phase per mode the building gadget
# registers. Every phase clicks the south face of a stone block at (0,150,0) from
# (0,150,4) facing north, with the range set by `/buildinggadgets2 range` and the
# mode by `/buildinggadgets2 mode`, and asserts the exact set of stone cells the
# click placed, plus one stone item consumed per placed cell. The expected sets are
# derived from the upstream Java (`util/modes/*.java`), not from the port's own
# planner, so a mode that answered the wrong shape fails on the cells, not just on
# a count.
#
#   GAME=<launcher-built horizon-client> tools/e2e.sh
#   GAME=$CONTROL_GAME tools/e2e.sh          # same Nightly, no package: the gadget
#                                            # id is unknown -> FAIL, EXIT:1
#   GAME=… SELFTEST=1 tools/e2e.sh           # every phase expects one cell too many
#                                            # and must reject it -> FAIL, EXIT:1
#
# The phases also assert the parsed action-bar reply of `/buildinggadgets2 mode`,
# `/buildinggadgets2 placeontop` and `/buildinggadgets2`, so the command path is
# covered by the value it prints, not by the absence of an error.
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
SOCK=${SOCK:-/tmp/opencode/buildinggadgets2-modes.sock}
SELFTEST=${SELFTEST:-0}
WORLD=${WORLD:-BuildingGadgets2Modes}
TOOL=buildinggadgets2:gadget_building
PHASES=9

[ -n "$GAME" ] || {
    echo "GAME must name a launcher-built horizon-client carrying buildinggadgets2.hmod" >&2
    echo "  ports/tools/launcher-game.sh build buildinggadgets2" >&2
    exit 1
}
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/buildinggadgets2-home.XXXXXX)
GAME_DIR=$SCRATCH/game
export HORIZON_HOME=$GAME_DIR
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
count_stone() { ctl inventory | jq -r '[.counts["minecraft:stone"] // 0] | add'; }
sneaking() { ctl state | jq -r '.player.sneaking'; }
actionbar() { ctl messages 4 | jq -r '.actionbar // ""'; }

# The stone cells the gadget placed in the stage, minus the clicked block, sorted.
placed() {
    ctl blocks -8 147 -8 8 166 8 \
        | jq -c '[.blocks[] | select(.[3] == "minecraft:stone")
                  | [.[0], .[1], .[2]] | select(. != [0,150,0])] | sort'
}

SELFTEST_FAILURES=0
# A phase assertion. A SELFTEST run records the rejection and carries on, so one
# run proves every phase's assertion can fail; the normal run stops at the first.
phase_fail() {
    if [ "$SELFTEST" = 1 ]; then
        echo "SELFTEST: $*"
        SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))
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

# Clear every cell a phase may place in, keeping the plank platform at y=149
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

# phase <name> <mode|default> <range> <on|off> <expected cells>
phase() {
    local name=$1 mode=$2 range=$3 on_top=$4 expected=$5
    local want expected_n before after actual delta reply

    echo "== phase $name: mode $mode, range $range, place-on-top $on_top =="
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
    if [ "$mode" = "horizontal_row" ]; then
        reply=$(command_reply "/buildinggadgets2")
        [ "$reply" = "Building Gadget: mode horizontal_row, range $range, block minecraft:stone, place-on-top off" ] \
            || phase_fail "$name: the status reply was '$reply'"
    fi

    select_block
    ctl give minecraft:stone 40 >/dev/null
    ctl wait 2
    ctl select "$TOOL" >/dev/null

    before=$(count_stone)
    ctl use-item
    ctl wait 6
    after=$(count_stone)
    delta=$((before - after))
    actual=$(placed)
    want=$(printf '%s' "$expected" | jq -c 'sort')
    expected_n=$(printf '%s' "$expected" | jq 'length')
    if [ "$SELFTEST" = 1 ]; then
        # Invert the expectation: one more cell and one more coordinate.
        expected_n=$((expected_n + 1))
        want=$(printf '%s' "$want" | jq -c '. + [[99,99,99]] | sort')
    fi
    echo "  placed: $actual"
    echo "  consumed: $delta stone"
    if [ "$actual" != "$want" ] || [ "$delta" != "$expected_n" ]; then
        phase_fail "$name: placed $actual (consumed $delta), expected $want (consumed $expected_n)"
    fi
}

echo "== launch (survival world $WORLD; the package is installed in the instance) =="
launch "$WORLD"
ctl wait-ready 120

echo "== a plank platform under the player at y=149; the clicked block floats at (0,150,0) =="
ctl command "time set noon"
ctl command "difficulty peaceful"
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

echo "== set the range to 5 (the clicked cell plus four); each phase sets its own =="
reply=$(command_reply "/buildinggadgets2 range 5")
echo "  reply: $reply"
[ "$reply" = "Building Gadget range set to 5" ] || fail "the range reply was '$reply'"

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

echo "== the status reply names the grid mode and the place-on-top setting =="
reply=$(command_reply "/buildinggadgets2")
echo "  reply: $reply"
[ "$reply" = "Building Gadget: mode grid, range 5, block minecraft:stone, place-on-top on" ] \
    || phase_fail "the final status reply was '$reply'"

if [ "$SELFTEST" = 1 ]; then
    [ "$SELFTEST_FAILURES" -ge "$PHASES" ] \
        || fail "SELFTEST: only $SELFTEST_FAILURES of $PHASES phase assertions rejected the wrong expectation"
    echo "FAIL: SELFTEST: all $PHASES phase assertions rejected a deliberately wrong expectation"
    exit 1
fi

echo "PASS: the carried recipe crafted one gadget with its ingredients consumed (a missing lapis matched nothing), and all $PHASES modes placed exactly the upstream cells and consumed one stone per cell"
