#!/usr/bin/env bash
# In-game check for Building Gadgets 2 slice 1 (the building gadget's
# horizontal row).
#
# The gadget is selected on a stone block (sneak right click) and its range is
# set to 6 with `/buildinggadgets2 range 6` (the pin has no scroll-wheel hook).
# A plain right click on that block's south face then builds the row away from
# the player: the clicked cell is solid (skipped), and the next five cells at
# z=-1..-5 must become stone. Exactly five stone items must be consumed.
#
#   tools/e2e.sh                     # 5 blocks placed, 5 stone consumed
#   COUNTERFACTUAL=1 tools/e2e.sh    # same clicks, no stone in the inventory:
#                                    # 0 placed, 0 consumed
#
# The counterfactual is how the check proves it can fail: the scenario and
# every assertion are unchanged, only the supply is withheld. If the port
# placed without paying, the counterfactual's zero assertion would fail.
#
# EMBEDDED=1 launches the game with **only the native bundle** (no `--dev-mod`):
# the content comes from the bundle's embedded copy (`mixins/buildinggadgets2/data/`,
# the AE2 pattern). The run checks the file the helper wrote is byte-identical
# to `content/buildinggadgets2.hmod`, and CONTROL=1 (default with EMBEDDED)
# first launches the same binary with `.horizon-cache` unusable, where the
# gadget id must be unknown.
set -euo pipefail

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/buildinggadgets2
# The control binary is shared at the campaign pin; the game is this port's build.
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/buildinggadgets2-e2e-run.sock}
COUNTERFACTUAL=${COUNTERFACTUAL:-0}
# SELFTEST=1 keeps the default run but expects six placed blocks from a 5-block
# gadget range, so the run must fail: the counterfactual proving the check can fail.
SELFTEST=${SELFTEST:-0}
EMBEDDED=${EMBEDDED:-0}
CONTROL=${CONTROL:-$EMBEDDED}
WORLD=${WORLD:-BuildingGadgets2E2E}
CONTROL_WORLD=BuildingGadgets2Control
TOOL=buildinggadgets2:gadget_building

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
[ -f "$PORT/content/horizon-mod.toml" ] || { echo "missing content mod: $PORT/content" >&2; exit 1; }

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
# `--game-dir` makes the game chdir into it, so the embedded-content helper's
# `.horizon-cache` (and HORIZON_HOME) is this folder, not the launch directory.
GAME_DIR=$SCRATCH/game
CACHE=$GAME_DIR/.horizon-cache
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

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    local args=(--game "$GAME" --world "$world" --seed 20261105 --headless --wait
                --game-dir "$GAME_DIR" --resources "$RES")
    if [ "$EMBEDDED" = 1 ]; then
        echo "  launch: no --dev-mod; the bundle carries its own content"
    else
        args+=(--dev-mod "$PORT/content")
    fi
    "$CTL" --socket "$SOCK" launch "${args[@]}"
}

# EMBEDDED=1: the bundle must have written its content out at startup, byte for
# byte the delivered content mod.
embedded_content_check() {
    local cache=$CACHE/buildinggadgets2-embedded-content.hmod
    [ -f "$cache" ] || fail "the bundle did not write its embedded content to $cache"
    cmp -s "$cache" "$PORT/content/buildinggadgets2.hmod" \
        || fail "the embedded content differs from content/buildinggadgets2.hmod"
    echo "  embedded content: $(wc -c < "$cache" | tr -d ' ') B at $cache, identical to content/buildinggadgets2.hmod"
}

# The control for EMBEDDED=1: the same binary (every hook is in it) with the
# embedded content made unavailable — `.horizon-cache` is a plain file, so the
# helper can neither compare nor write its content file and falls back to
# `Continue`. The gadget id must then be unknown; if it resolves, the run
# proves nothing about the embedded file.
control_leg() {
    echo "== control: the same game with no embedded content file =="
    rm -rf "$CACHE"
    mkdir -p "$GAME_DIR"
    : > "$CACHE"
    launch "$CONTROL_WORLD"
    ctl wait-ready 120
    ctl command "item replace entity @s hotbar.0 with $TOOL 1" >/dev/null 2>&1 || true
    ctl wait 4
    [ "$(count_item "$TOOL")" = "0" ] \
        || fail "the control game holds $TOOL, so the content did not come from the embedded file"
    [ -f "$CACHE" ] \
        || fail "the control's .horizon-cache was replaced: the helper wrote its content"
    echo "  without the embedded file: no $TOOL in the inventory"
    ctl quit >/dev/null 2>&1 || true
    rm -f "$CACHE"
}

echo "== launch (survival world $WORLD, $([ "$EMBEDDED" = 1 ] && echo 'one bundle, no --dev-mod' || echo 'content --dev-mod content/')) =="
if [ "$CONTROL" = 1 ]; then control_leg; fi
launch "$WORLD"
ctl wait-ready 120
if [ "$EMBEDDED" = 1 ]; then embedded_content_check; fi

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
ctl give "$TOOL" 1 >/dev/null || fail "the content mod did not provide $TOOL"
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
elif [ "$EMBEDDED" = 1 ]; then
    echo "PASS: one bundle, no --dev-mod: the embedded content gave the gadget, a 5-block row away from the clicked face cost exactly 5 stone, and the control showed the gadget id unknown without the embedded file"
else
    echo "PASS: the building gadget placed a 5-block row away from the clicked face and consumed 5 stone"
fi
