#!/usr/bin/env bash
# In-game check for Building Gadgets 2 — the held build, the `buildinggadgets2:deny` tag and the
# world-border edges.
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
# Three deny-tag phases close the tag family:
#
#   * `deny_selection` — the carried `buildinggadgets2:deny` refuses a sneak click on a member
#     (`minecraft:piston_head`: destroy speed 1.5, no fluid, so only the tag can refuse it) with
#     upstream's `Invalid Block`, and leaves the previous selection in the component;
#   * `deny_destination_control` — two `minecraft:snow` cells at grid destinations are ordinary
#     replaceable cells under the carried tag: 8 cells would be allowed;
#   * `deny_destination` — the same two cells after a world data pack re-declares the tag with
#     `minecraft:snow` and the server reloads: 6 cells would be allowed. The pair is the proof:
#     a read that always answers `false` reports 8 in the tagged run, one that always answers
#     `true` reports 0 in the control run (and in every mode phase).
#
# Two clean-state phases close the selection family (upstream's `GadgetUtils.cleanBlockState`):
#
#   * `clean_selection` — a lit furnace's transient `lit=true` is stored at the block's default
#     (`lit=false`) while `facing=west` survives; the status reply names the stored state string;
#   * `clean_transient` — a ripe `minecraft:wheat[age=7]` is stored as `age=0`.
#
# Two destruction-gadget phases close the destination-plan family (slice 0.2.7):
#
#   * `destroy_refusals` — a sneak click beside the clicked stone plans only the stone:
#     a `minecraft:piston_head` (carried deny tag member) and a `minecraft:bedrock`
#     (`destroy_speed -1`) in the planned layer are both excluded;
#   * `destroy_control` — the same rig with two ordinary stones plans all three.
#
#   The pair makes an always-false and an always-true tag/speed read fail. The parsed
#   `/buildinggadgets2` reply names the count, and neither rig removes a block. `SELFTEST`
#   inverts the count in both directions (1->3 and 3->1).
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
DESTROY_TOOL=buildinggadgets2:gadget_destruction
PHASES=18
HELD="Building Gadget: held (blocked: no-cancellable-placement)"
# A carried `buildinggadgets2:deny` member whose other selection clauses all pass, so only the
# tag can refuse it: destroy speed 1.5 (> 0), no fluid, block item air.
TAG_BLOCK=minecraft:piston_head
# The plate faces the player (west), so its west face is a full 1x1 target
# (26.3 outline [0,0,0 ... 0.25,1,1] for this facing); a north-facing head's west
# face is open space and the click would hit nothing.
TAG_BLOCK_STATE='minecraft:piston_head[facing=west,short=false,type=normal]'
# A replaceable block, so the destination rule is the only thing that can refuse it once the
# test world's data pack names it in the tag.
DENYABLE_BLOCK=minecraft:snow

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

# Aim at a block's face and sneak right click it, as a player selecting a block does.
select_at() { # x y z face
    ctl look-at-block "$1" "$2" "$3" "$4" >/dev/null || fail "cannot aim at $1 $2 $3 $4"
    ctl wait 1
    select_block
}

# What block is at a single cell, as a block id (`minecraft:air` when empty): `ctl blocks`
# reports the state string, so its property list is cut off.
block_at() { # x y z
    ctl blocks "$1" "$2" "$3" "$1" "$2" "$3" \
        | jq -r '(.blocks[0][3] // "minecraft:air") | split("[")[0]'
}

# The full state string at a cell (`minecraft:oak_door[facing=west,half=upper,…]`), as the engine
# serializes it: a phase asserts the state it is about to click, not one it hopes is there.
state_at() { # x y z
    ctl blocks "$1" "$2" "$3" "$1" "$2" "$3" | jq -r '.blocks[0][3] // "?"'
}

# One property's value out of a state string, without depending on the engine's property order.
state_property() { # state name
    local list=${1#*[} pair
    list=${list%]}
    local IFS=,
    for pair in $list; do
        case "$pair" in
            "$2="*) echo "${pair#*=}"; return 0 ;;
        esac
    done
    echo "?"
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

# The deny tag's selection rule (upstream's `GadgetBuilding.onShiftAction` →
# `GadgetUtils.isValidBlockState`): a sneak click on a carried member answers
# `Invalid Block` and leaves the previous selection alone; stone still selects. The two
# expectations swap in a SELFTEST run, so each rejects.
deny_selection_phase() {
    local name=deny_selection
    local reply at want_stone="Selected minecraft:stone" want_tagged="Invalid Block"
    local want_status before after spent actual

    echo "== deny tag phase $name: a $TAG_BLOCK cannot be selected, stone can =="
    PHASE_REJECTED=0
    reset_stage
    ctl select "$TOOL" >/dev/null
    # At eye level, two blocks east of the player: the west face is what the crosshair
    # reaches, with nothing between it and the eye.
    ctl setblock 2 151 4 "$TAG_BLOCK_STATE" >/dev/null
    ctl wait 2
    at=$(block_at 2 151 4)
    echo "  deny-tagged block at 2 151 4: $at"
    [ "$at" = "$TAG_BLOCK" ] \
        || phase_fail "$name: the deny-tagged block is '$at', expected $TAG_BLOCK (a state that breaks cannot test the tag)"

    reply=$(set_mode grid)
    [ "$reply" = "Building Gadget mode set to grid" ] \
        || phase_fail "$name: the mode reply was '$reply'"
    reply=$(command_reply "/buildinggadgets2 range 5")
    [ "$reply" = "Building Gadget range set to 5" ] \
        || phase_fail "$name: the range reply was '$reply'"
    reply=$(set_place_on_top off)
    [ "$reply" = "Building Gadget place-on-top off" ] \
        || phase_fail "$name: the place-on-top reply was '$reply'"

    if [ "$SELFTEST" = 1 ]; then
        local swap=$want_stone
        want_stone=$want_tagged
        want_tagged=$swap
    fi

    select_at 0 150 0 south
    reply=$(actionbar)
    echo "  selection reply (stone): $reply"
    [ "$reply" = "$want_stone" ] \
        || phase_fail "$name: selecting stone answered '$reply', expected '$want_stone'"

    select_at 2 151 4 west
    reply=$(actionbar)
    echo "  selection reply ($TAG_BLOCK): $reply"
    [ "$reply" = "$want_tagged" ] \
        || phase_fail "$name: selecting $TAG_BLOCK answered '$reply', expected '$want_tagged'"

    reply=$(command_reply "/buildinggadgets2")
    echo "  status: $reply"
    want_status="Building Gadget: mode grid, range 5, block minecraft:stone, place-on-top off, placement held (blocked: no-cancellable-placement)"
    [ "$reply" = "$want_status" ] \
        || phase_fail "$name: the status reply was '$reply', expected '$want_status' (the refused click must not change the selection)"

    ctl look-at-block 0 150 0 south >/dev/null || fail "cannot aim at the clicked block"
    ctl wait 1
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
    assert_held "$name" 8 "$actual" "$spent" "$reply"
}

# The deny tag's destination rule: the grid's eight destinations are air except two cells that
# hold `minecraft:snow`, a replaceable block. Under the carried tag snow is ordinary (8); once
# the test world's data pack names it in the tag, those two cells are refused (6).
deny_destination_phase() {
    local name=$1 want=$2
    local reply before after spent actual snow_a snow_b

    echo "== deny tag phase $name: snow at 2 of the grid's 8 destinations, $want would be allowed =="
    PHASE_REJECTED=0
    reset_stage
    ctl setblock 6 150 6 "$DENYABLE_BLOCK" >/dev/null
    ctl setblock -6 150 -6 "$DENYABLE_BLOCK" >/dev/null
    ctl wait 2
    snow_a=$(block_at 6 150 6)
    snow_b=$(block_at -6 150 -6)
    echo "  destinations (6,150,6) and (-6,150,-6): $snow_a, $snow_b"
    [ "$snow_a" = "$DENYABLE_BLOCK" ] && [ "$snow_b" = "$DENYABLE_BLOCK" ] \
        || phase_fail "$name: the two destination cells are '$snow_a' and '$snow_b', expected $DENYABLE_BLOCK"

    reply=$(set_mode grid)
    [ "$reply" = "Building Gadget mode set to grid" ] \
        || phase_fail "$name: the mode reply was '$reply'"
    reply=$(command_reply "/buildinggadgets2 range 5")
    [ "$reply" = "Building Gadget range set to 5" ] \
        || phase_fail "$name: the range reply was '$reply'"
    reply=$(set_place_on_top off)
    [ "$reply" = "Building Gadget place-on-top off" ] \
        || phase_fail "$name: the place-on-top reply was '$reply'"

    select_at 0 150 0 south
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

# A world data pack that re-declares `buildinggadgets2:deny` with `minecraft:snow`, so the destination
# rule is the only thing that can decide the two snow cells. A pack *adds* to a tag (the world's files
# and the mod's are a union — the 0.2.6 probe measured that an empty `{"values":[]}` leaves the
# carried members in place), which is why this leg extends the tag rather than replacing it.
# `/reload` rebuilds the world's tables from its packs (`Game::reload_world_tables`), which is what
# a `TagsUpdatedEvent` does upstream.
install_deny_extension() {
    local pack=$GAME_DIR/saves/$WORLD/datapacks/denyextension
    mkdir -p "$pack/data/buildinggadgets2/tags/block"
    cat > "$pack/pack.mcmeta" <<'JSON'
{"pack":{"pack_format":121,"description":"buildinggadgets2 e2e: the deny tag with a replaceable member"}}
JSON
    cat > "$pack/data/buildinggadgets2/tags/block/deny.json" <<JSON
{"values":["$DENYABLE_BLOCK"]}
JSON
    echo "== the test world declares $DENYABLE_BLOCK in the deny tag, then /reload =="
    echo "  data pack: $pack"
    ctl command "reload" >/dev/null
    ctl wait 6
}

# The selection's `GadgetUtils.cleanBlockState`: a sneak click stores the block's default state with
# only the properties upstream's `DENY_PROPERTIES` does not name. A lit furnace carries both kinds —
# `lit` is denied, `facing` is kept — so one click proves both sides of the split. The clicked state
# is read and asserted first, so the expectation cannot hold vacuously.
#
# The gap line's door example cannot be selected at all since 0.2.5: the carried
# `buildinggadgets2:deny` tag names `#minecraft:doors`, and a world data pack can only *add* to a
# tag (a pack that re-declares the tag as `{"values":[]}` left the doors refused, measured in the
# probe run). The door's own `half`/`open`/`hinge`/`powered` cleaning is pinned by the unit test
# `cleaning_a_door_drops_half_and_open_and_keeps_facing_and_hinge`; the game assertion uses a block
# the tag lets through.
clean_selection_phase() {
    local name=clean_selection reply at want
    local tail='place-on-top off, placement held (blocked: no-cancellable-placement)'
    local raw clean

    echo "== clean phase $name: a lit furnace's lit=true is dropped, facing=west survives =="
    PHASE_REJECTED=0
    reset_stage
    ctl select "$TOOL" >/dev/null
    reply=$(set_mode grid)
    [ "$reply" = "Building Gadget mode set to grid" ] \
        || phase_fail "$name: the mode reply was '$reply'"
    reply=$(command_reply "/buildinggadgets2 range 5")
    [ "$reply" = "Building Gadget range set to 5" ] \
        || phase_fail "$name: the range reply was '$reply'"
    reply=$(set_place_on_top off)
    [ "$reply" = "Building Gadget place-on-top off" ] \
        || phase_fail "$name: the place-on-top reply was '$reply'"

    # A full cube (the crosshair target is unambiguous) with one denied and one kept property, both
    # off their defaults, and no tick or neighbour rule that would rewrite `lit` before the click.
    ctl setblock 2 151 4 'minecraft:furnace[facing=west,lit=true]' >/dev/null
    ctl wait 3
    at=$(state_at 2 151 4)
    echo "  furnace at 2 151 4: $at"
    [ "$(state_property "$at" lit)" = true ] \
        || phase_fail "$name: the furnace is '$at', expected lit=true (an unlit furnace proves nothing)"
    [ "$(state_property "$at" facing)" = west ] \
        || phase_fail "$name: the furnace is '$at', expected facing=west"

    select_at 2 151 4 west
    reply=$(actionbar)
    echo "  selection reply: $reply"
    [ "$reply" = "Selected minecraft:furnace" ] \
        || phase_fail "$name: selecting the furnace answered '$reply'"
    reply=$(command_reply "/buildinggadgets2")
    echo "  status: $reply"

    # 0.2.5 stored the clicked state as it was read (the differential); upstream's cleanBlockState
    # stores `lit` at the block's default and carries `facing` over.
    raw="Building Gadget: mode grid, range 5, block $at, $tail"
    clean="Building Gadget: mode grid, range 5, block minecraft:furnace[facing=west,lit=false], $tail"
    want=$clean
    if [ "$SELFTEST" = 1 ]; then want=$raw; fi
    [ "$reply" = "$want" ] \
        || phase_fail "$name: the status reply was '$reply', expected '$want'"
}

# A crop's `age` is one of the eight denied `IntegerProperty` instances (`AGE_7`, eight values), so
# the stored state is the block's default. The raw state is the 0.2.5 differential.
clean_transient_phase() {
    local name=clean_transient reply at want
    local tail='place-on-top off, placement held (blocked: no-cancellable-placement)'
    local raw clean

    echo "== clean phase $name: a ripe wheat's age=7 is transient, the default age=0 is stored =="
    PHASE_REJECTED=0
    reset_stage
    ctl select "$TOOL" >/dev/null
    reply=$(set_mode grid)
    [ "$reply" = "Building Gadget mode set to grid" ] \
        || phase_fail "$name: the mode reply was '$reply'"
    reply=$(command_reply "/buildinggadgets2 range 5")
    [ "$reply" = "Building Gadget range set to 5" ] \
        || phase_fail "$name: the range reply was '$reply'"
    reply=$(set_place_on_top off)
    [ "$reply" = "Building Gadget place-on-top off" ] \
        || phase_fail "$name: the place-on-top reply was '$reply'"

    ctl setblock 2 150 4 minecraft:farmland >/dev/null
    ctl setblock 2 151 4 'minecraft:wheat[age=7]' >/dev/null
    ctl wait 3
    at=$(state_at 2 151 4)
    echo "  wheat at 2 151 4: $at"
    [ "$(state_property "$at" age)" = 7 ] \
        || phase_fail "$name: the wheat is '$at', expected age=7 (a plant that never ripened proves nothing)"

    select_at 2 151 4 west
    reply=$(actionbar)
    echo "  selection reply: $reply"
    [ "$reply" = "Selected minecraft:wheat" ] \
        || phase_fail "$name: selecting the wheat answered '$reply'"
    reply=$(command_reply "/buildinggadgets2")
    echo "  status: $reply"

    raw="Building Gadget: mode grid, range 5, block $at, $tail"
    clean="Building Gadget: mode grid, range 5, block minecraft:wheat[age=0], $tail"
    want=$clean
    if [ "$SELFTEST" = 1 ]; then want=$raw; fi
    [ "$reply" = "$want" ] \
        || phase_fail "$name: the status reply was '$reply', expected '$want'"
}

# The destruction gadget's destination plan. `rejected` puts a deny-tag member and an
# unbreakable block beside the clicked stone; `control` puts two ordinary stones there.
# The parsed status count is the assertion, and the rig blocks are read back afterwards so
# the slice cannot pass by quietly removing them.
destroy_phase() {
    local name=$1 want=$2
    local reply cell_a cell_b plan_want status_want held
    local material_a=minecraft:piston_head material_b=minecraft:bedrock

    echo "== destruction phase $name: two cells beside the clicked stone, $want planned =="
    PHASE_REJECTED=0
    reset_stage
    # The plan is one whole y=150 layer: clear it first so the count cannot include a
    # block another phase left behind.
    ctl fill -8 150 -8 8 150 8 minecraft:air >/dev/null
    ctl fill 0 150 0 0 150 0 minecraft:stone >/dev/null
    if [ "$name" = "destroy_control" ]; then
        material_a=minecraft:stone
        material_b=minecraft:stone
    fi
    ctl setblock 1 150 0 "$material_a" >/dev/null
    ctl setblock 2 150 0 "$material_b" >/dev/null
    ctl wait 3
    cell_a=$(block_at 1 150 0)
    cell_b=$(block_at 2 150 0)
    echo "  cells (1,150,0) and (2,150,0): $cell_a, $cell_b"
    [ "$cell_a" = "$material_a" ] && [ "$cell_b" = "$material_b" ] \
        || phase_fail "$name: the rig cells are '$cell_a' and '$cell_b', expected $material_a and $material_b"

    # A give lands in the first free slot of the whole inventory; earlier phases can leave
    # the hotbar full, so clear it first — `ctl select` only accepts a hotbar slot.
    ctl command "clear @s" >/dev/null
    ctl wait 2
    ctl give "$DESTROY_TOOL" 1 >/dev/null || fail "the package did not provide $DESTROY_TOOL"
    ctl wait 2
    ctl select "$DESTROY_TOOL" >/dev/null || fail "cannot select $DESTROY_TOOL"
    ctl wait 2
    held=$(ctl inventory | jq -r '.held.item // "none"')
    [ "$held" = "$DESTROY_TOOL" ] \
        || phase_fail "$name: the held item is $held, expected $DESTROY_TOOL"

    # The range belongs to the destruction gadget's own component, so set it while it is held.
    reply=$(command_reply "/buildinggadgets2 range 5")
    [ "$reply" = "Building Gadget range set to 5" ] \
        || phase_fail "$name: the range reply was '$reply'"

    select_at 0 150 0 south
    reply=$(actionbar)
    echo "  plan reply: $reply"
    plan_want="Destruction Gadget: planned $want cells, 0 removed (blocked: no-cancellable-placement)"
    if [ "$want" -eq 1 ]; then
        plan_want="Destruction Gadget: planned 1 cell, 0 removed (blocked: no-cancellable-placement)"
    fi
    [ "$reply" = "$plan_want" ] \
        || phase_fail "$name: the plan reply was '$reply', expected '$plan_want'"

    reply=$(command_reply "/buildinggadgets2")
    echo "  status: $reply"
    status_want="Destruction Gadget: range 5, planned $want cells, 0 removed (blocked: no-cancellable-placement)"
    if [ "$SELFTEST" = 1 ]; then
        # Invert the count in both directions: the refused rig claims 3, the control rig 1.
        if [ "$name" = "destroy_control" ]; then
            status_want="Destruction Gadget: range 5, planned 1 cells, 0 removed (blocked: no-cancellable-placement)"
        else
            status_want="Destruction Gadget: range 5, planned 3 cells, 0 removed (blocked: no-cancellable-placement)"
        fi
    fi
    [ "$reply" = "$status_want" ] \
        || phase_fail "$name: the status reply was '$reply', expected '$status_want'"

    # The plan is a plan only: neither rig lost a block.
    cell_a=$(block_at 1 150 0)
    cell_b=$(block_at 2 150 0)
    [ "$cell_a" = "$material_a" ] && [ "$cell_b" = "$material_b" ] \
        || phase_fail "$name: the plan removed a block ($cell_a, $cell_b)"
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

echo "== the destruction gadget id resolves only with the package =="
if ctl give "$DESTROY_TOOL" 1 >/dev/null 2>&1; then
    ctl wait 2
    got=$(ctl inventory | jq -r '[.counts["'"$DESTROY_TOOL"'"] // 0] | add')
    if [ "$got" -ge 1 ]; then
        echo "  $DESTROY_TOOL resolved (packaged game), count $got"
    else
        fail "the destruction gadget id $DESTROY_TOOL was not added to the inventory"
    fi
    ctl command "clear @s" >/dev/null
    ctl wait 2
else
    fail "the destruction gadget id $DESTROY_TOOL is unknown (control game without the package)"
fi

echo "== the carried gadget recipe crafts in a real crafting table =="
ctl setblock 3 150 4 minecraft:crafting_table
ctl wait 2
python3 "$PORT/tools/craft_gadget.py" "$SOCK" "$CTL" 3 150 4 \
    || fail "the carried gadget recipe did not craft (or its control matched)"

echo "== the carried destruction gadget recipe crafts in a real crafting table =="
ctl setblock 3 150 4 minecraft:crafting_table
ctl wait 2
python3 "$PORT/tools/craft_destruction.py" "$SOCK" "$CTL" 3 150 4 \
    || fail "the carried destruction gadget recipe did not craft (or its control matched)"
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

deny_selection_phase
deny_destination_phase deny_destination_control 8
install_deny_extension
deny_destination_phase deny_destination 6

# The selection's cleaned state, on blocks the carried deny tag lets through.
clean_selection_phase
clean_transient_phase

# The destruction gadget's destination plan: two rigs whose counts differ by exactly the
# refused pair (the deny-tag piston head and the unbreakable bedrock).
destroy_phase destroy_refusals 1
destroy_phase destroy_control 3


if [ "$SELFTEST" = 1 ]; then
    [ "$REJECTED_PHASES" -ge "$PHASES" ] \
        || fail "SELFTEST: only $REJECTED_PHASES of $PHASES phases rejected the wrong expectation ($SELFTEST_FAILURES assertions)"
    echo "FAIL: SELFTEST: all $PHASES phases rejected a deliberately wrong expectation ($SELFTEST_FAILURES assertions)"
    exit 1
fi

echo "PASS: both carried recipes crafted one gadget each with their ingredients consumed (a missing lapis matched nothing in each); all 9 modes held the build with the exact upstream cell count, 0 blocks written and 0 stone spent; the world border's upper edge refused the cell exactly on it while allowing it one cell inside; the carried buildinggadgets2:deny tag refused a sneak click on minecraft:piston_head (leaving minecraft:stone selected) while its destination rule counted 8 cells with ordinary snow and 6 once the test world named snow in the tag; the selection stored upstream's cleaned state: a lit furnace kept facing=west and dropped lit=true, and a ripe wheat was stored as age=0; and the destruction gadget's parsed status named 1 planned cell with a deny-tag piston head and a -1-destroy-speed bedrock beside the click, 3 with two ordinary stones there, 0 removed in both, with the RenderBlock line recorded blocked(no-render-block-surface)"
