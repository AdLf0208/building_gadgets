#!/usr/bin/env bash
# In-game check for Building Gadgets 2 slice 1 (the building gadget's
# horizontal row).
#
# The gadget is selected on a stone block (sneak right click), its range is set to
# 6 with `/buildinggadgets2 range 6` (a player command, so it arrives with a player
# source; the console has no held gadget), and a plain right click on that block's
# south face then builds the row away from the player: the clicked cell is solid
# (skipped), and the next five cells at z=-1..-5 must become stone. Exactly five
# stone items must be consumed.
#
#   GAME=<launcher-built horizon-client> tools/e2e.sh
#   GAME=… COUNTERFACTUAL=1 tools/e2e.sh    # same clicks, no stone in the
#                                          # inventory: 0 placed, 0 consumed
#   GAME=… SELFTEST=1 tools/e2e.sh          # expects six placed blocks from a
#                                          # five-block range: must FAIL
#
# The counterfactual is how the check proves it can fail: the scenario and every
# assertion are unchanged, only the supply is withheld. If the port placed without
# paying, the counterfactual's zero assertion would fail. SELFTEST is the same
# proof on the default run (an assertion that must not hold).
#
# The SDK package is one flat `.hmod` that carries both halves (code in `src/`, the
# gadget's assets/data at the root), so the game must be built **by the launcher**
# with this package installed — there is no `--dev-mod` content folder to load and
# no embedded-content helper any more:
#
#   horizon-launcher native-build <this port> --instance <instance> --trust-native-code
#
# then point GAME at that instance's `horizon-client`. `CTL` is the shared control
# binary for the same game build (the launcher's instance has its own `horizon-ctl`
# too; either speaks the same protocol).
set -euo pipefail

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/buildinggadgets2
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/buildinggadgets2-e2e-run.sock}
COUNTERFACTUAL=${COUNTERFACTUAL:-0}
SELFTEST=${SELFTEST:-0}
WORLD=${WORLD:-BuildingGadgets2E2E}
TOOL=buildinggadgets2:gadget_building

[ -n "$GAME" ] || {
    echo "GAME must name a launcher-built horizon-client carrying buildinggadgets2.hmod" >&2
    echo "  horizon-launcher native-build $PORT --instance <i> --trust-native-code" >&2
    exit 1
}
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
[ -f "$PORT/horizon-mod.toml" ] || { echo "missing the SDK package: $PORT" >&2; exit 1; }

if [ "$COUNTERFACTUAL" = 1 ]; then
    STONE=0
    EXPECT_PLACED=0
    EXPECT_CONSUMED=0
else
    STONE=64
    EXPECT_PLACED=5
    EXPECT_CONSUMED=5
    if [ "$SELFTEST" = 1 ]; then
        EXPECT_PLACED=6
    fi
fi

SCRATCH=$(mktemp -d /tmp/opencode/buildinggadgets2-home.XXXXXX)
GAME_DIR=$SCRATCH/game
export HORIZON_HOME=$GAME_DIR
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
# The game's game folder defaults to the working directory: run from scratch so
# saves/options/caches never land in the port tree.
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
count_item() { ctl inventory | jq -r --arg i "$1" '[.counts[$i] // 0] | add'; }
# The built row, exactly: z=-5..-1 at x=0, y=100 (the clicked block is z=0).
row_stone() { ctl blocks -1 100 -5 1 100 -1 | jq '[.blocks[] | select(.[3] == "minecraft:stone")] | length'; }
sneaking() { ctl state | jq -r '.player.sneaking'; }

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
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    "$CTL" --socket "$SOCK" launch --game "$GAME" --world "$world" --seed 20261105 \
        --headless --wait --game-dir "$GAME_DIR" --resources "$RES"
}

echo "== launch (survival world $WORLD; the package is installed in the instance) =="
launch "$WORLD"
ctl wait-ready 120

echo "== a stone platform in open air (y=99), clicked block at (0,100,0) =="
ctl command "time set noon"
ctl command "difficulty peaceful"
ctl command "kill @e[type=!minecraft:player,type=!minecraft:item]"
ctl command "clear @s"
ctl fill -6 99 -6 6 99 6 minecraft:stone
ctl fill -6 100 -6 6 104 6 minecraft:air
ctl fill 0 100 0 0 100 0 minecraft:stone
ctl tp 0 100 2
ctl wait 2
ctl look-at-block 0 100 0 south || fail "cannot aim at the clicked block's south face"
ctl wait 2

if [ "$STONE" -gt 0 ]; then
    ctl give minecraft:stone "$STONE" >/dev/null || fail "cannot give stone"
fi
ctl give "$TOOL" 1 >/dev/null || fail "the package did not provide $TOOL"
ctl wait 2
[ "$(count_item "$TOOL")" -ge 1 ] || fail "$TOOL is missing from the inventory"
ctl select "$TOOL" >/dev/null || fail "cannot select $TOOL"
ctl wait 2
held=$(ctl inventory | jq -r '.held.item // "none"')
echo "held: $held"
[ "$held" = "$TOOL" ] || fail "the held item is $held, expected $TOOL"

echo "== set the range to 6 (the clicked cell plus five row cells) =="
ctl chat "/buildinggadgets2 range 6"
ctl wait 5
ctl messages 5 | jq -r '.action_bar // empty'

echo "== sneak right click selects the stone block =="
ctl key shift down
ctl wait 2
[ "$(sneaking)" = "true" ] || fail "the player is not sneaking after 'key shift down'"
ctl use-item
ctl wait 5
ctl key shift up
ctl wait 2
[ "$(sneaking)" = "false" ] || fail "the player is still sneaking after 'key shift up'"

before=$(count_item minecraft:stone)
row_before=$(row_stone)
echo "stone before: $before"
echo "row stone before: $row_before"

echo "== right click the clicked block's south face with $TOOL (range 6) =="
ctl use-item
ctl wait 10

after=$(count_item minecraft:stone)
row_after=$(row_stone)
echo "stone after: $after"
echo "row stone after: $row_after"
ctl blocks -1 100 -5 1 100 -1 | jq -c '{counts, blocks}'

[ "$before" -ge "$EXPECT_CONSUMED" ] || fail "not enough stone to consume"
[ $((row_after - row_before)) -eq "$EXPECT_PLACED" ] \
    || fail "the click placed $((row_after - row_before)) blocks, expected $EXPECT_PLACED"
[ $((before - after)) -eq "$EXPECT_CONSUMED" ] \
    || fail "the click consumed $((before - after)) stone, expected $EXPECT_CONSUMED"
[ "$(ctl block 0 100 0 | jq -r '.block')" = "minecraft:stone" ] \
    || fail "the clicked block itself was replaced"

if [ "$COUNTERFACTUAL" = 1 ]; then
    echo "PASS (counterfactual, no stone in the inventory): 0 blocks placed, 0 stone consumed"
else
    echo "PASS: the building gadget placed a 5-block row away from the clicked face and consumed 5 stone"
fi
