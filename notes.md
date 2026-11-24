# Building Gadgets 2 — port notes (P3 logic tier)

Ported from **Building Gadgets 2** (`Direwolf20-MC/BuildingGadgets2`, **MIT**)
for Minecraft Java 1.21.1 NeoForge. Local-only until the maintainer approves
publishing (ARCHITECTURE.md §8).

Status: `ported-partial` — **slice 2 (the mode family)** implemented: one building
gadget, sneak-click block selection, and all eight of upstream's building modes
(`build_to_me` by default, `vertical_wall`, `vertical_column`, `surface`,
`horizontal_wall`, `stairs`, `horizontal_row`, `grid`), selected with
`/buildinggadgets2 mode <name>`, range 1..15, `place_on_top`, one item consumed per
placed block. The anchor, undo, energy, bound inventories, the mode wheel and the
GUI/tooltip, and the other gadgets (paste/rotate/cut/destruction/exchange) are
later slices.

> **0.2.4 holds the build.** A review item (protection must be able to cancel one
> destination) cannot be met by this pin's API: there is no placement call that names
> the acting player *and* lets a protection mod veto one position. A plain right click
> therefore writes nothing and says so on the action bar
> (`blocked: no-cancellable-placement`); selection, the eight mode plans, the range and
> the commands stay live. The symbols checked and the issue shape that lifts the hold
> are in [`## Review repair (0.2.4)`](#review-repair-024--the-placement-hold-and-the-borders-upper-edge).

**Version 0.2.0 — the SDK package (history).** This port was migrated on 2026-11-18 from
the retired `.hnmod` container to the flat SDK `.hmod` (Horizon's mod-API merge,
#16941): one file carries the code (`src/`) **and** the carried content
(`assets/`, `data/`), instead of an `.hnmod` with a content `.hmod` embedded in
`mixins/`. `src/runtime.rs` is **byte-identical** to the pre-migration file
(sha256 `069556ad84ac040bdb945027feb225f80d10222f0a62f78970b59bdc266303f2`),
tests included. The migration recipe, the file map and every trap are in
[`## Migration to the SDK format`](#migration-to-the-sdk-format).

## Slice 2 — the mode family (2026-11-21, version 0.2.3)

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

**Gap it closes.** `### Gaps (current)`: "**Mode fixed to `horizontal_row`** (upstream's default is
`build_to_me`); the other modes, mode switching (the wheel is client-only) … are later slices."

**What shipped.** All eight modes `GadgetModes.setupDefaultModes` registers for the building gadget,
each translated from its own class in `util/modes/`:

| Mode | Upstream class | Rule as translated |
|---|---|---|
| `build_to_me` (default) | `BuildToMe` | cells strictly between the clicked block and the player along the face's axis, capped at 32 |
| `vertical_wall` | `VerticalWall` | a wall across the face's perpendicular horizontal axis and ±range/2 vertically |
| `vertical_column` | `VerticalColumn` | along the clicked face on a top/bottom face; ±range/2 up and down on a side face |
| `surface` | `Surface` | the square one block past the face, kept where the block behind is the clicked block's state |
| `horizontal_wall` | `HorizontalWall` | a square on a top/bottom face; on a side face a wall along the face, ±range/2 sideways |
| `stairs` | `Stairs` | a staircase stepping away from the face (or the facing), climbing or descending with the player |
| `horizontal_row` | `HorizontalRow` | the 0.1.0 row, unchanged |
| `grid` | `Grid` | a lattice on the clicked plane, spacing `((range+1-2) % 6) + 2`, bound `(range+1)*7/5` |

The default is upstream's `build_to_me` (`GadgetNBT.getMode` searches for that id first), so a fresh
gadget now builds between the click and the player instead of a row.

**Selection.** The radial mode wheel is a client screen and is not ported. The mode is named on the
command line — `/buildinggadgets2 mode <name>`, accepting the full id (`buildinggadgets2:stairs`) or the
path (`stairs`) — and the `PLACE_ON_TOP` toggle with `/buildinggadgets2 placeontop on|off`. Both live in
the same `buildinggadgets2:gadget_state` component (`mode`, `place_on_top`) beside the range and the
selection, so they travel with the stack. `/buildinggadgets2` now reports mode, range, block and
place-on-top.

**Upstream quirks kept, and why.** `HorizontalWall` gates a side-face column on its **centre** cell: a
blocked centre skips the whole column, even when its sides are free (`horizontal_wall` test
`horizontal_wall_skips_a_column_whose_centre_is_blocked`). `Stairs` uses its own sign rule
(`EAST`/`SOUTH` → −1), the opposite of `MagicHelpers.invertOnFace`, so it steps *away* from the click
where the wall grows *toward* it; both are pinned by tests. These are design quirks, not defects, so the
port reproduces them and the e2e asserts the resulting cells.

**Red → green.** The 11 new mode tests were run against a dispatch stub whose new arms answered
`Vec::new()` — 19 passed, 11 failed, e.g.

```
---- runtime::tests::grid_lays_a_lattice_on_the_clicked_plane stdout ----
thread 'runtime::tests::grid_lays_a_lattice_on_the_clicked_plane' panicked at src/runtime.rs:1096:9:
assertion `left == right` failed
  left: []
 right: [[-6, 100, -6], [-6, 100, 0], [-6, 100, 6], [0, 100, -6], [0, 100, 6], [6, 100, -6], [6, 100, 0], [6, 100, 6]]
test result: FAILED. 19 passed; 11 failed; 0 ignored; 0 measured; 0 filtered out
```

(`ports/_shared/evidence/buildinggadgets2/sdk-test-red.log`, `EXIT:101`.) The eight collectors then made
them pass: **30 passed, 0 failed**.

**In-game leg.** `tools/e2e.sh`, one `--offscreen` launch: the carried recipe is crafted in a real
crafting table (and the same pattern without lapis matches nothing), then one phase per mode. Each phase
asserts the **exact cell set** the click placed (derived from the Java, not from the port) and one stone
consumed per placed cell, and the command replies are read from the action bar. `SELFTEST=1` gives every
phase one cell too many and carries on, so one run proves **all nine** phase assertions reject a wrong
expectation.

### Coverage — every family in the jar

| Family | Status |
|---|---|
| `build_to_me` | tested — phase `build_to_me`, 3 cells at `z=1..3`, 3 stone |
| `vertical_wall` | tested — phase `vertical_wall`, 24 cells (5×5 minus the clicked one), 24 stone |
| `vertical_column` | tested — phase `vertical_column`, 4 cells around the click, 4 stone |
| `surface` | tested — phase `surface`, the single backed cell `(0,150,1)`, 1 stone |
| `horizontal_wall` | tested — phase `horizontal_wall` (range 3), 3×3 cells, 9 stone |
| `stairs` | tested — phase `stairs`, 4 rising cells, 4 stone |
| `horizontal_row` | tested — phase `horizontal_row`, 4 cells, 4 stone |
| `grid` | tested — phase `grid` (8 cells) and `grid_on_top` (9 cells one block up) |
| `place_on_top` | tested — `grid_on_top` and the `horizontal_wall` top-face unit test |
| mode selection + status | tested — `mode`/`range`/`placeontop`/status replies asserted in the log |
| carried recipe | tested — crafted in a table; ingredients consumed; missing-lapis control empty |
| gadget item | tested — crafted, given, selected, held; unknown in the control game |
| stone block | placed and read back in every phase; the clicked block is never replaced |
| anchor, undo, energy | not ported — no anchor/undo list and no FE hook on the gadget |
| preview, per-tick queue, retry | not ported — placement is immediate (see the gaps) |
| bound inventory / AE2 / Curios | not ported — payment walks the main inventory |
| mode wheel, GUI, tooltip | not ported — the mode is a command; the screen is client work |
| paste/rotate/cut/destruction/exchange gadgets | not ported — later slices |

### Verification (0.2.3)

| Check | Result |
|---|---|
| `cargo test` | **30 passed**, 0 failed (`sdk-test.log`) |
| `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| `cargo fmt --check` | exit 0 (`sdk-fmt.log`) |
| `cargo horizon-mod build` twice, `rm -f Cargo.lock` before each | **37,953 B**, sha256 `eed5258a…9920`, `cmp` identical |
| `cargo horizon-mod check` | `buildinggadgets2 0.2.3 (native-mod, format native-1/0a875da…, for 26.3)`, exit 0 |
| `fits --build 0a875da` | `0a875da  made for` (`sdk-fits.log`) |
| `fits --build 0b14f8a` | `0b14f8a  fits` (`sdk-fits-other.log`) — a different build, so the pin run is not a tautology |
| `verify-sdk-package.py buildinggadgets2` | 1/1 |
| e2e (modded) | `PASS` + `EXIT:0`, recipe + 9 mode phases (`e2e-modes.log`) |
| e2e (control, same Nightly without the package) | `FAIL: the carried gadget recipe did not craft…` + `EXIT:1` |
| e2e (`SELFTEST=1`) | 9/9 phase assertions rejected, `FAIL` + `EXIT:1` |
| `check-e2e-logs.py buildinggadgets2-e2e --strict` | 5/5 |
| `check-skills.py --notes ports/buildinggadgets2` | `Skills:` line present |

## Behaviour implemented (slice 1)

- `buildinggadgets2:gadget_building` (stack size 1) right-clicked on a block:
  - **sneak + right click** selects the clicked block's state (upstream's
    `onShiftAction`); air, unbreakable and non-source fluids are refused;
  - **plain right click** builds the selected block's **horizontal row**:
    `side = face.axis == Y ? player facing : -face`, cells
    `clicked + side*i` for `i in 0..range`, skipping cells that are not
    replaceable, outside the build height, or where the state cannot survive;
  - each placed cell consumes one item of the block's kind; creative pays
    nothing; a row stops when the supply runs out (the last block is rolled
    back if the take races);
  - range defaults to **1** (upstream) and is set with
    `/buildinggadgets2 range <1..15>` (the pin has no scroll-wheel/key hook).
- `/buildinggadgets2` and `/buildinggadgets2 range` report the range and the
  selected block on the action bar.

## Hooks

| Hook | Helper | What it does |
|---|---|---|
| `use_item_on` (v1, HEAD) | `use_on.rs` | Sneak selects the clicked block; a plain click builds the row and claims the use. |
| `register_commands` (v1, HEAD) | `register.rs` | Declares `/buildinggadgets2 [range [n]]`. |
| `perform_command` (v1, HEAD) | `perform.rs` | Reads/stores the player's range and answers on the action bar. |

*(The 0.1.0 hooks, for the record. The current SDK methods and the host calls that
replaced each old `Game::` helper are in
[`## Migration to the SDK format`](#migration-to-the-sdk-format).)*

## Gaps (slice 1)

- **Mode fixed to `horizontal_row`.** Upstream's default is `build_to_me`; the
  other modes (vertical wall/column, surface, horizontal wall, stairs, grid,
  build-to-me) and mode switching are deferred. The mode wheel is client-only.
- **Selection and range are per-player in memory**, not item components: the
  pin's `use_item_on` gives `&ItemStack` and has no component-write path, and
  there is no scroll/key hook. They reset on relog.
- **Selection stores the raw state id**, so transient properties (age, lit,
  waterlogged, open, ...) are not cleaned as upstream's `cleanBlockState`
  does; orientation properties are preserved. ~~The `buildinggadgets2:deny` tag
  is not consulted (no tag read is wired).~~ **Closed in 0.2.5** — Slice 4.
- **Placement is immediate** through `Game::native_set_block`: no render
  block/animation, no per-tick queue, no retry pass for supports placed later,
  no `BlockItem.place`/`setPlacedBy`, and block-entity data is not carried.
- **No energy.** The pin has no FE hook; upstream also charges 50 FE/block.
  Only items are paid.
- No anchor, undo list, bound inventory/AE2/Curios sourcing, `place_on_top`
  toggle, connected-area, GUI/tooltip, sounds, particles or chunk-protection
  (`mayInteract` has no pin API).
- Item payment uses `native_count_items`/`native_take_items` (plain stacks
  only), so component-bearing stacks are not counted.

## In-game check

`tools/e2e.sh` (survival, Overworld, content loaded with `--dev-mod content/`):

1. give 64 stone and the gadget; hold the gadget;
2. `/buildinggadgets2 range 6` (the clicked cell plus five row cells);
3. sneak right click the stone block at `(0,100,0)`'s south face to select it;
4. plain right click the same face: the row runs north through the clicked
   block (skipped, still stone) and places **exactly 5** stone at
   `z=-1..-5`, consuming **exactly 5** stone.

`COUNTERFACTUAL=1` runs the identical scenario with **no stone in the
inventory**: 0 blocks placed and 0 consumed. That is how the check proves it
can fail — a port that ignored the supply would place 5 and fail the
counterfactual's zero assertion. If `/buildinggadgets2 range 6` did not apply,
range would stay 1 and the default run would place 0, so the command path is
covered too.

## Upstream source

Analysed 2026-11-05. Repository
`https://github.com/Direwolf20-MC/BuildingGadgets2` (MIT), branch **`1.21.1`**,
commit **`11cd2efb2719e480018a81cfe5a05bd087cb1388`** (2026-10-24, tag
`v21.1.1`; the ATM10 pack ships 1.3.9, the same 1.21.1 line at a newer
commit). `main` is the 26.1.2 line (`529c2962…`, tag `v26.1.2.3`); its
`HorizontalRow.java` is the same rule, read to confirm the behaviour did not
change. Full findings:
[`FINDINGS.md`](FINDINGS.md).

Areas read: `setup/{Registration,ModSetup,Config,BG2DataComponents}.java`,
`common/items/{BaseGadget,GadgetBuilding}.java`,
`util/modes/{BaseMode,HorizontalRow,BuildToMe,VerticalColumn}.java`,
`util/{GadgetNBT,GadgetUtils,BuildingUtils}.java`,
`common/events/{ServerTickHandler,ServerBuildList}.java`,
`common/network/handler/PacketRangeChange.java`,
`client/screen/ModeRadialMenu.java`, `client/events/EventKeyInput.java`,
`datagen/{BG2Recipes,BG2LanguageProvider}.java`, the item model/texture and
the Patchouli `mode_horz_row` entry.

The source fixes these facts for the rewrite:

- **Range** is `GadgetNBT.getToolRange`, default **1**, slider **1..15**; the
  scroll key sends `RangeChangePayload`.
- **Side** is `hitSide.axis == Y ? player.getDirection() :
  hitSide.getOpposite()`; the row scans `i in 0..range` from the clicked block
  (or one above with `place_on_top`) and keeps valid cells, skipping invalid
  ones without stopping.
- **Validity** (building): `canBeReplaced`, inside the build height, the
  selected state `canSurvive`, and `mayInteract`.
- **Payment** is the block's own drop/clone item (`getDropsForBlockState`,
  silk-touch gadget first); survival simulates the removal, then the real
  removal happens on placement, main inventory before a bound inventory,
  Curios and the offhand. Creative pays nothing.
- **Energy** is 500000 max / 50 per block; the build stops when it runs out.
- **Placement** is queued through `ServerTickHandler` (1..50 cells/tick),
  renders a `RenderBlock` first, then writes the real state; a failed
  `canSurvive` is retried once.

## Verification results (in-game)

Proof of slice 1 at the pin `c9d474dd9116e84c243f8992d75c11314df56f06`
(2026-11-05).

### Runtime and lint (standalone library)

```
$ cargo fmt --check                                                   exit 0
$ cargo clippy --all-targets --locked --offline -- -D warnings       exit 0, 0 warnings
$ cargo test --locked --offline -p horizon-buildinggadgets2
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
                                                                      exit 0
```

### Staged source and embedded tests

`HORIZON_STAGE=/tmp/opencode/buildinggadgets2-stage` was used so a concurrent
campaign-wide `clean-targets.sh` run could not delete this port's stage (the
script deletes every `ports/*/target/mixins`).

```
$ python3 tools/horizon.py stage --horizon <campaign>/horizon
{ "stage": "/tmp/opencode/buildinggadgets2-stage/modded", "stage_changed": true,
  "horizon_revision": "c9d474dd9116e84c243f8992d75c11314df56f06",
  "staging_seconds": 1587.18 }                                       exit 0
```

The staged workspace then ran the same commands `horizon.py check/test/build`
run (the wrapper re-stages on every call, 25+ minutes under the campaign load):

```
$ cargo check --locked --offline -p horizon-client --bin horizon-client \
    --no-default-features --target-dir <port>/target/horizon-cargo
    Finished `dev` profile ... in 13m 17s                            exit 0, 0 warnings

$ cargo test --locked --offline -p horizon-client --bin horizon-client \
    --no-default-features --target-dir <port>/target/horizon-cargo \
    game::buildinggadgets2::tests -- --nocapture
running 10 tests
test game::buildinggadgets2::tests::a_bottom_face_uses_the_player_facing ... ok
... (10/10)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured;
             531 filtered out                                          exit 0
```

### Build

```
$ cargo build --profile fast --locked --offline -p horizon-client \
    --bin horizon-client --no-default-features --target-dir <port>/target/horizon-cargo
    Finished `fast` profile [optimized] target(s) in 35m 32s          exit 0
$ cargo build --profile fast --locked --offline -p horizon-client \
    --bin horizon-ctl --no-default-features --target-dir <port>/target/horizon-cargo
    Finished `fast` profile [optimized] target(s) in 9.62s            exit 0
```

The e2e used these `fast` binaries (the script's defaults). A debug build of
the same source also succeeded earlier (`horizon-client` 7m 25s, `horizon-ctl`
14.35s); the debug run produced the same 5/5 and 0/0 results.

### Real run — exit 0

`timeout 900 tools/e2e.sh` → exit 0 (the script's defaults:
`target/horizon-cargo/fast/horizon-ctl` and
`.../fast/horizon-client`). Survival Overworld, 13×13 stone platform at
`y=99`, clicked block at `(0,100,0)`, player at `(0,100,2)` aiming at its
south face.

```
held: buildinggadgets2:gadget_building
== set the range to 6 (the clicked cell plus five row cells) ==
== sneak right click selects the stone block ==
stone before: 64
row stone before: 0
== right click the clicked block's south face with buildinggadgets2:gadget_building (range 6) ==
stone after: 59
row stone after: 5
{"counts":{"minecraft:stone":5},"blocks":[
  [0,100,-5,"minecraft:stone"],[0,100,-4,"minecraft:stone"],
  [0,100,-3,"minecraft:stone"],[0,100,-2,"minecraft:stone"],
  [0,100,-1,"minecraft:stone"]]}
PASS: the building gadget placed a 5-block row away from the clicked face and consumed 5 stone
```

The row runs north (away from the player) from the clicked block, exactly as
`HorizontalRow` does; the clicked cell is solid and skipped, and the five cells
beyond it are the placed line. Exactly five stone left the inventory.

### Counterfactual — exit 0, and it proves the check can fail

`COUNTERFACTUAL=1 ... tools/e2e.sh` runs the identical scenario with **no stone
in the inventory**:

```
held: buildinggadgets2:gadget_building
stone before: 0
row stone before: 0
== right click the clicked block's south face with buildinggadgets2:gadget_building (range 6) ==
stone after: 0
row stone after: 0
{"counts":{},"blocks":[]}
PASS (counterfactual, no stone in the inventory): 0 blocks placed, 0 stone consumed
```

Same clicks, same range, same selected block: zero placements and zero
consumption. If the port placed without paying, this run would place five and
fail the zero assertion, so the check is fail-capable. The range command is
also covered: without `/buildinggadgets2 range 6` the range would stay at 1 and
the default run would place 0.

### Delivered content

`python3 tools/package_content.py` (deterministic zip, fixed timestamps):

```
{ "file": "content/buildinggadgets2.hmod", "size": 6034,
  "sha256": "5890eec2f4e386c6e79da4ab8a015adac126427b08866c3ad6ff391e20d19049" }
$ cargo-horizon-mod horizon-mod check content/buildinggadgets2.hmod --json
{"ok":true,...,"size":6034,"version":"1.3.9",...}                    exit 0
```

The archive holds 9 files: the manifest, the item declaration, the 26.3 item
model definition, the upstream model and 16×16 texture, the recipe, the
recipe-unlock advancement, the creative tab and `en_us.json`.

## Licence

MIT (upstream `LICENSE`, Copyright (c) 2023 Direwolf20-MC). The content and
code are local-only until the maintainer approves publishing.

## Re-pin to Horizon `1d63e0a` and artifact rebuild (2026-11-09)

Mode A migration (the port pinned `83ad384f` and shipped only the `.hmod`);
the `.hnmod` half is new. The stamped hook catalog is byte-identical between
the two pins, so the five injections, two modules and every binding are
unchanged.

| | Before | After |
|---|---|---|
| Horizon | `83ad384f48d0742746d518737511882680aab5b7` | **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`** |
| horizon-mixins | `e8cd0aa38cffef49cb2e088ef716baa107148c02` | **`5a64af938ec86b00d53d16a408aabc0038fb3e57`** |
| `game.rs` SHA-256 | `996432b9f7f9461fa69c90ff798daf6b89fe3795f379d7c6f401916633296463` | `64813cf4db0b9c3983d42908609bb633b9fbbc705f0c890396f940a6c7cef80f` |

### Ladder (through `ports/tools/build-slot.sh`, `CARGO_INCREMENTAL=0`)

| Step | Result |
|---|---|
| `cargo fmt` + `--check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0, **0 warnings** |
| `cargo test --locked --offline` | **11 passed**, 0 failed |
| `horizon.py stage` | exit 0 at `1d63e0a` |
| `horizon.py check` | `cargo_exit: 0`, **0 warnings** (60 s) |
| `horizon.py test` | **11 passed**, 1240 filtered (151 s) |
| `horizon.py build` (fast) | `cargo_exit: 0` (524 s) |

Logs: `ports/_shared/evidence/buildinggadgets2/`.

### Delivered artifacts (reproducible)

| File | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `content/buildinggadgets2.hmod` | 6,034 B | `96d4e5c7379e10fb7d5dc97e31c740e38c7018b77fd85c96113122cdf769b367` | 9 |
| `buildinggadgets2.hnmod` (content embedded) | 16,472 B | `781f4e5438e8daf38f6660246114834a7a5e3e716bf0f50e399b663f72a3ed0c` | 11 |

Both are now built by the pin's own `cargo-horizon-mod` (the `.hmod`, checked
with `horizon-mod check` → `no code: content only`) and `tools/package.py` (the
`.hnmod`); the earlier `.hmod` came from the port's own zip helper, so its bytes
changed. A second `tools/package.py` run reproduced the `.hnmod` hash.

### In-game e2e (fast build, shared `horizon-ctl`)

| Leg | Verdict |
|---|---|
| default `tools/e2e.sh` | `PASS` + `EXIT:0` (`e2e.log`) |
| `SELFTEST=1 tools/e2e.sh` (expects six placed blocks from a 5-block range) | `FAIL: the click placed 5 blocks, expected 6` + `EXIT:1` (`e2e-negative.log`) |
| one file only (`EMBEDDED=1`, no `--dev-mod`) | `PASS` + `EXIT:0` (`e2e-embedded.log`); the control leg shows the gadget id unknown without the embedded file |
| embed fail-capability (`EMBEDDED=1 SELFTEST=1`) | `FAIL: the click placed 5 blocks, expected 6` + `EXIT:1` (`e2e-embedded-selftest.log`) |

`python3 ports/tools/check-e2e-logs.py buildinggadgets2 --strict` → 4/4 ok.

## Embedded content (AE2 pattern)

The registry stores **one file per project** (playbook §1.0), so a port with both
halves could not be listed faithfully. This bundle now carries its content,
exactly as the merged AE2 entry does:

- **`mixins/buildinggadgets2/data/buildinggadgets2-content.hmod`** is the built
  `content/buildinggadgets2.hmod`, byte for byte (6,034 B, `96d4e5c7…b367`).
  `tools/package.py` packs the bundle's top-level `.rs`/`.toml` **and**
  everything under `data/` as `mixins/<id>/data/…`, with fixed zip timestamps,
  and refuses a `data/<id>-content.hmod` that is not byte-identical to
  `content/<id>.hmod`, so a rebuilt content mod can never ship stale.
- **`mixins/buildinggadgets2/embedded_content.rs`** with the `embedded-content`
  `[[injection]]` (`target = "horizon_client::game::mods::content_root"`,
  `kind = "HEAD"`, `cancellable = true`, bindings `root`/`enabled`/`dev`)
  compiles the file in with `native_data!` (API 41), writes it to
  `.horizon-cache/buildinggadgets2-embedded-content.hmod` when the copy on disk
  differs, appends it to the dev-mod list and returns its `content_root`. The
  helper calls `content_root` once more and stops at its own
  `dev.contains(&file)` guard (AE2's one-level recursion). An unwritable cache
  logs a warning and `Continue`s (no content) instead of failing the game.
  Because this port launches with `--game-dir`, the game chdirs into that folder,
  so the cache the helper uses is `<game-dir>/.horizon-cache` — the e2e reads
  that path, not the launch directory.

Verified in game with **only the bundle** (`EMBEDDED=1`, no `--dev-mod`): the
whole scenario passes and the file the helper wrote is byte-identical to
`content/buildinggadgets2.hmod`. The control (`CONTROL=1`) launches the same
binary with `.horizon-cache` replaced by a plain file: `item replace … with
buildinggadgets2:gadget_building 1` gives nothing, so the id is unknown without
the embedded file — the PASS is not vacuous.

### In-game transcript (quoted from `e2e-embedded.log`)

```
== control: the same game with no embedded content file ==
  without the embedded file: no buildinggadgets2:gadget_building in the inventory
  launch: no --dev-mod; the bundle carries its own content
  embedded content: 6034 B at /tmp/opencode/buildinggadgets2-home.XXXX/game/.horizon-cache/buildinggadgets2-embedded-content.hmod, identical to content/buildinggadgets2.hmod
held: buildinggadgets2:gadget_building
stone before: 64
row stone before: 0
stone after: 59
row stone after: 5
PASS: one bundle, no --dev-mod: the embedded content gave the gadget, a 5-block row away from the clicked face cost exactly 5 stone, and the control showed the gadget id unknown without the embedded file
EXIT:0
```

The same scenario with `SELFTEST=1` ends `FAIL` + `EXIT:1`
(`e2e-embedded-selftest.log`).

## Submission gate

`python3 ports/tools/upstream_ready.py buildinggadgets2` exits **0** (`READY`):
the split content+code blocker is cleared because the `.hmod` now travels
**inside** the `.hnmod` (`mixins/buildinggadgets2/data/buildinggadgets2-content.hmod`,
AE2 pattern, proved in game from the one file), so the registry's one-file rule
is met. Pin, licence, artifact in the port (not `target/`), `Skills:` line and
the `PASS` + `EXIT:0` e2e are green.

Skills: superpowers/using-superpowers,verification-before-completion,systematic-debugging; BeautifulRust/rust-best-practices

## Review repairs (PR #603, 2026-11-11)

Review: `ports/_shared/horizon-mods/reviews/pr-603-buildinggadgets2.md` (CHANGES_REQUESTED, 2 items + source follow-up).

| Reviewer's item | This job |
|---|---|
| 1. relative link and the porter-only author list | link absolute and `authors` already named Direwolf20-MC; re-checked |
| 2. artifact not staged | **orchestrator** (no write access) |
| follow-up: the page says selection/range are per-player in memory and reset on relog, but the bundle stores `buildinggadgets2:gadget_state` and declares `mouse_scrolling`/`scroll.rs` | **fixed**: the description now says the state is an item component that survives a relog and that the mouse wheel changes the range through the `mouse_scrolling` hook |

The description was also rewritten in full: the sneak-select / plain-build behaviour, the
row maths, the creative rule, the fixed mode, the raw-state selection and the remaining
gadget families.

Artifact: `buildinggadgets2.hnmod` **17,933 B** `9ac67e640ffb0e169e22e2771cc827ea080c397bfb6e6579a2f43d645a2a883f`
(content `buildinggadgets2.hmod`).

## Migration to the SDK format (2026-11-18, 0.2.0)

*Migrated from the retired `.hnmod` container to the flat SDK package, following
`ports/imfast/notes.md` → `## Migration to the SDK format` (the verified reference:
12 steps, 15 traps). Only the glue, the manifest and the packaging were rewritten;
`src/runtime.rs` and its 11 tests carried over byte-for-byte.*

**Why.** Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) retired
`.hnmod` on 2026-11-18: the registry closed every `.hnmod` submission with one
reason (*"republish this as a native `.hmod` with the SDK from the next Nightly"*),
and an `.hnmod` no longer loads. The new unit is **one flat `.hmod`** that carries
content and code together — which is what this port needed, since its 0.1.0
bundle faked that by embedding a content `.hmod` inside `mixins/` (the AE2
pattern). That embedding is gone: `assets/` and `data/` now sit at the package
root and the launcher loads both halves from the one file.

**Pins.** SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18),
mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`** (moved from `5a64af93…`),
launcher 0.2.78, `cargo-horizon-mod` from `horizon-rust/horizon@main`.

**Tool.** `cargo horizon-mod` runs **directly in the port** with
`CARGO_INCREMENTAL=0`, no `--target-dir`, no staging: `ports/tools/build-slot.sh`
speaks the retired `.hnmod` tool (`tools/horizon.py stage|check|test|build`), which
this layout no longer has, so the slot registry does not apply to an SDK package.
The trade-off is that these steps are not journaled in the slot registry; they cost
seconds (a test build is 2.5 s, the package is a zip), not the 25+ minutes staging cost.

### File map

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and identity: `id`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` pins |
| `Cargo.toml` | one crate; the only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks`: `register`, `component_types`, `register_commands`, `run_mod_command`, `mouse_scrolling` |
| `src/support.rs` | the game glue: the component, the held-stack read/write, the inventory count/take, the `runtime::World` impl, placement, the commands, the wheel |
| `src/runtime.rs` | the pure planner and command parser, **copied byte-for-byte** (`cp`, then `cmp`): sha256 `069556ad84ac040bdb945027feb225f80d10222f0a62f78970b59bdc266303f2` |
| `assets/buildinggadgets2/` | the gadget's item definition, model, 16×16 texture and `en_us.json` (moved from `content/assets/`) |
| `data/buildinggadgets2/` | the item behaviour row (`horizon/item/gadget_building.json`: `template minecraft:stick`, `max_stack_size 1`), recipe, recipe-unlock advancement, creative tab (moved from `content/data/`) |
| `.gitignore` | `/target`, `/.horizon`, `/Cargo.lock` (trap 5) |
| `LICENSE`, `LICENSE-NOTICE.md`, `icon.png` | packed into the `.hmod`; the publish tree is not the package |
| `buildinggadgets2.hmod` | the delivered package (18 entries, in the port root) |
| `tools/e2e.sh` | kept; its launch was rewritten for a launcher-built game (no `--dev-mod`, no embedded-content leg) |
| `notes.md`, `FINDINGS.md`, `VALIDATION.md`, `REPIN-AUDIT.md`, `README.md` | kept (README rewritten for the SDK layout) |

Removed into `ports/_shared/evidence/buildinggadgets2/retired-hnmod-0.1.0/`
(read-only evidence, never built again):

| Deleted | Why |
|---|---|
| `mixins/buildinggadgets2/` (`horizon-mixin.toml`, `runtime.rs` [moved to `src/`], `support.rs`, `use_on.rs`, `register.rs`, `perform.rs`, `scroll.rs`, `components.rs`, `embedded_content.rs`, `data/`) | the container is retired; the seven helpers are now one `Hooks` impl plus the glue in `src/support.rs` |
| `buildinggadgets2.hnmod` (17,933 B, `9ac67e64…`) | unloadable on launcher ≥ 0.2.70 |
| `compatibility.json` | `[native]` in the manifest is the pin now |
| `rust-toolchain.toml` | the pinned Horizon commit and the launcher decide the toolchain (no such file in a package) |
| `tools/horizon.py`, `tools/package.py`, `tools/package_content.py`, `tools/package.sh`, `tools/notice.py`, `tools/test_stage.py`, `tools/test_template.py`, `tools/audit/` | staging and `.hnmod` packaging are gone; `cargo horizon-mod build` packs the package |
| `content/` (`horizon-mod.toml`, `README.md`, `buildinggadgets2.hmod` 6,034 B `96d4e5c7…`, `target/`) | its `assets/` and `data/` moved to the port root; the content-only zip is superseded by the one package |
| `Cargo.lock` | not needed without registry dependencies, and packing one changes the artifact (trap 5) |
| `AGENTS.md` | it described the retired `.hnmod` workflow ("never run a bare cargo here"), which this layout contradicts; `AGENTS.md` at the campaign root governs |

### Helper → hook mapping (0.1.0 injection → 0.2.0 SDK)

| 0.1.0 injection (helper) | 0.2.0 method | Mapping notes |
|---|---|---|
| `use_item_on` v1 HEAD (`use_on.rs`) → `buildinggadgets2_state::on_use_on` | `#[mod_api::horizon_part] impl ItemPart for Gadget { fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> }`, attached in `Hooks::register` with `registrar.item_part(registrar.item_key(runtime::GADGET), Gadget)` | the Part is **item-scoped**, so the old `if &*stack.item != GADGET` check and the `stack.count == 0` check disappear; `claimed = true` becomes `Some(InteractionResult::SuccessServer)`; the client half answers `None` so its prediction never writes the world |
| `register_commands` v1 HEAD (`register.rs`) | `Hooks::register_commands(host, event)`: push `mod_api::Command::new(usage)` (same three usages) | `NativeCommand` → `mod_api::Command` |
| `perform_command` v1 HEAD (`perform.rs`) | `Hooks::run_mod_command(host, event) -> Flow` over `RunModCommandEvent { source, command }`, `Flow::Cancel` when taken | **not** `player_command`: only `run_mod_command` gives a live `CommandSource::Player { player }` and a parsed `CommandInvocation`; `player_command` documents that player helpers reach nobody there. The dispatcher resolves arguments, so `src/support.rs::command_line` rebuilds the line (`literals` + the integer argument) and hands it to the tested `runtime::parse_command` |
| `component_types` v1 HEAD (`components.rs`) | `Hooks::component_types(context, event)`: `event.types.push(...)` | unchanged shape (`Vec<String>`) |
| `mouse_scrolling` v1 HEAD (`scroll.rs`) | `Hooks::mouse_scrolling(host: &mut dyn ClientHost, event) -> Flow` | same client hook; the held stack now comes from `host.local_player()` + `PlayerRef::selected_slot`/`inventory_slots`, and the server call is `host.send_chat("/buildinggadgets2 range N")` (v1 `native_send_chat`) |
| `game.native_set_block(&dimension, cell, state, flags::UPDATE_ALL)` | **`LevelRef::set_block(host, pos, state, BlockUpdateFlags::NEIGHBORS \| BlockUpdateFlags::CLIENTS)`** | this is the per-cell placement call: `UPDATE_ALL` is the two flags, `Ok(true)` = placed, `Ok(false)`/`Err` = refused (unloaded, out of height, unchanged) |
| `game.native_count_items` / `native_take_items` (plain stacks only) | `PlayerRef::inventory_slots(host)` filtered to the main inventory (`slot < 36`) by `ItemKey`, then `PlayerRef::set_inventory_slot(host, slot, …)` to take | explicit instead of a helper, and it **counts component-bearing stacks** the old helpers could not see; equipment and the off hand are not payment, as upstream |
| `game.held()` / `game.held_mut()` | `PlayerRef::selected_slot` + `inventory_slots` for the read, and `ItemStack::set` + `PlayerRef::set_inventory_slot` for the write | the hook hands `&ItemStack`, so the component write is on a copy and the slot is set back |
| `game.native_component` / `native_set_component` | `ItemStack::get(&GadgetStateComponent)` / `ItemStack::set(&…, Tag::compound(…))` with a local `impl mod_api::Component { type Value = Tag; id/decode/encode }` | the descriptor is three methods; no derive, no codegen |
| `game.native_feedback(Text)` | `PlayerRef::action_bar(host, Text)` for a player source, `host.commands().feedback(Text)` for the console | the action bar is what 0.1.0 used |
| `game.res.facts().get(state)` (`name`, `hardness`, `fluid`, `fluid_level`, `item`, `replaceable`) | `host.block_states()`: `BlockMetadata::name`, `BlockStates::destroy_speed` (`-1` = unbreakable), `BlockStates::replaceable`; `level.fluid_state(host, pos)` (`FluidState::source`, `amount`); `host.block_item(state.block())` | the fluid source rule is the same (a non-source fluid is refused) |
| `game.pack.assets.registry.state(id)` / `default_state("minecraft:air")` | `host.block_state_id("minecraft:air")` / `block_state_id(state_string)` | the state is stored as its **serialized string** (`block_state_string`), which is what vanilla's block-state codec writes into a component and what survives a version change; `BlockStateId` was a session-local number |
| `game.player.shift_key_down` | `PlayerRef::is_secondary_use_active(host)` | vanilla's `isSecondaryUseActive()` (sneak or a secondary-use item) — the closest read on a server handle; see the deviations |
| `game.player.yaw` | `EntityRef::rotation(host)` for `event.player_entity_id`, then `runtime::facing_from_yaw` | server-side read, no client field |
| `game.player.game_mode == 3` | `PlayerRef::is_spectator(host)` | same gate (spectators build nothing) |
| `game.with_level(… behaviours.can_survive …)` | **nothing** — see the gap below | `canSurvive` is a `BlockPart` hook a mod answers for *its own* blocks; a port cannot ask another block's answer |
| `game.player.dimension` | `PlayerRef::level(host)` | a `LevelRef` handle, which the write goes through |

### Placement: what replaced the old helper, and previews

- **The host call that replaced `Game::native_set_block` is
  `LevelRef::set_block(self, host: &mut dyn ServerHost, pos: [i32; 3], state: BlockState, flags: BlockUpdateFlags) -> Result<bool, LevelWriteError>`**
  (generated from `LevelWriteHost::set_block`; documented as `Level.setBlock`).
  The port calls it once per planned cell with
  `NEIGHBORS | CLIENTS` — the old `flags::UPDATE_ALL` — so neighbours and clients
  are told exactly as before, and the block entity of whatever it replaces goes as
  it did. `Ok(false)` covers "unchanged, unloaded or out of height"; an `Err` names
  a stale level, a non-authoritative side or an invalid state; both stop the row.
- The bulk alternative in the same area is
  `host.world_edit().edit_blocks(dimension, &BlockEdit { palette, filters, runs })`:
  one call for many cells, `fill … strict` semantics (block entities handled, whole
  chunks sent), **no neighbour notification** and no per-cell answer. It is the
  wrong shape for this mod: the row must stop when the supply runs out and must
  update neighbours (a torch or a gravel column would behave differently), and the
  port pays per cell. `wind_charge`-style bulk effects are where it belongs.
- **A per-block preview is *not* an API gap** — it is not ported. The pin has
  `Hooks::render_level_stage` (client, listen, per stage: `AfterSky`,
  `AfterOpaqueBlocks`, `AfterTranslucentBlocks`, `AfterWeather`, `AfterLevel`) and
  `WorldDrawContext::draw_model_instances`, so the planned cells can be drawn as
  translucent block models, which is what upstream's `RenderBlock` amounts to. It is
  absent for the same reason in 0.1.0 (immediate placement, no queue and no
  animation): a preview plus the `ServerBuildList` queue (1..50 cells/tick, one
  retry on a failed `canSurvive`) is a behaviour slice of its own, not a substrate
  the migration removed. The gap list below records it as such.

### Gaps (current, 0.2.3)

- **`canSurvive` is no longer pre-checked.** 0.1.0 asked the game's own
  `behaviours.can_survive(level, ticks, at, state)` before placing; at this pin
  `canSurvive` exists only as a `BlockPart` hook a mod answers for *its own*
  blocks (`CanSurviveEvent`), and there is no host query for another block's
  answer. `World::can_place` is therefore "loaded, inside the build height and
  replaceable", and a state that cannot stand where it is written (a torch on air)
  is written and then removed by the game's own shape updates, as `/setblock`
  behaves — where 0.1.0 skipped the cell. **Issue shape:** *"A host query for
  `BlockState.canSurvive(state, pos)` (the answer the game's own placement tests),
  so a builder can skip a cell instead of writing and losing it."*
- **Mode wheel not ported.** All eight modes exist and are selected with
  `/buildinggadgets2 mode <name>` (the radial menu that opens them upstream is a
  client screen). The other gadgets' modes (paste, cut, exchanges) are their own
  slices.
- **Selection stores the whole state string**, so transient properties (age, lit,
  waterlogged, open, …) are not cleaned as upstream's `cleanBlockState` does;
  orientation properties are preserved. (0.1.0 stored the numeric state id, which
  was session-local; the string is the codec-faithful form and survives a relog.)
- **Placement is immediate**: no preview, no per-tick queue, no retry pass for
  supports placed later, no `BlockItem.place` / `setPlacedBy`, block-entity data of
  the placed state is not carried, no sound or particles.
- **No energy** (upstream also charges 50 FE/block; the port pays items only).
- **No anchor and no undo.** Upstream's anchor block (build around a bound anchor
  instead of the click) and its undo list (restore the replaced states and refund
  the items) are not ported; placement does not remember what it replaced.
- No bound inventory / AE2 / Curios sourcing, connected-area fill, or GUI/tooltip.
  Chunk protection is covered by the per-cell `may_edit` gate (player build
  permission, `mayUseItemAt`, the world border and a dedicated server's
  spawn-protection radius); a mod's own protection hook has no pin query.
- Payment walks the main inventory explicitly, so it **does** see component-bearing
  stacks (the old `native_count_items` did not); that is a fix, not a gap.

### Deviations from 0.1.0 (deliberate)

1. **Sneak detection** is vanilla's `isSecondaryUseActive()` (shift key **or** a
   secondary-use item such as a shield) instead of the raw shift flag, because a
   server handle exposes no raw key state. Holding a shield and right-clicking a
   block with the gadget therefore selects instead of building; upstream's
   `onShiftAction` reads the same vanilla call, so this is closer to it than 0.1.0.
2. **The component's `selected` is a string** (see above), not an int; a gadget
   from a 0.1.0 world reads as "no selection" rather than a stale number.

### Traps the next port must know (in addition to the reference's 15)

16. **`use mod_api::prelude::*` was not enough** for this glue. `host.block_states()`
    needed `use mod_api::HostBlockStates`, and `BlockStates::name` needed
    `BlockMetadata`; but `block_state_id`, `block_state_string`, `block_item`,
    `player`, `entity` and `local_player` resolved on the host trait objects with no
    import at all, and importing `PlayersHost`/`EntitiesHost`/`MenusClientHost`
    produced unused-import warnings (which `-D warnings` turns into a failure). Add
    imports only when the compiler names the trait.
17. **`Compound` is a `BTreeMap<JavaString, Tag>`**, so `compound.get("range")` wants
    `&JavaString`. Match the tag (`let Some(tag @ Tag::Compound(_)) = …`) and call
    `Tag::get(&str)`, which is the helper that takes a `&str`; do not build
    `JavaString`s for reads.
18. **`JavaString: From<&str>` only** — there is no `From<String>`. Write
    `Tag::String(text.as_str().into())`.
19. **`doc_lazy_continuation`**: a `//!` line that starts with `+` (as in "sneak
    + right click") is parsed as a markdown list item and fails clippy. Reword it.
20. **`cargo fmt` formats `src/runtime.rs` too.** It stayed byte-identical only
    because the retired file was already rustfmt-clean; `cmp` against the retired
    copy after EVERY fmt run, before recording the byte-identity claim.
21. **The component descriptor can hold a `Tag`.** `impl mod_api::Component {
    type Value = Tag; id/decode/encode }` is three methods, so a mod's own component
    needs no derive and no codegen; `ItemStack::get/set` then read/write it, and
    `component_types` still declares the id.
22. **There is no `held_mut`.** A hook hands `&ItemStack`, so writing a component is
    `ItemStack::set` on a copy plus `PlayerRef::set_inventory_slot(host,
    selected_slot, Some(stack))` — hence `write_held` has to find the slot first.
23. **`use_item_on` (global, block-scoped) is not the successor of the old
    `use_item_on`.** The item-scoped `ItemPart::use_on` is, and it deletes the
    "is the stack the gadget" check because the game only asks it for that item.
24. **`perform_command` → `run_mod_command`, not `player_command`.** Only
    `run_mod_command` carries `CommandSource::Player { player }` and the parsed
    `CommandInvocation`; `player_command` explicitly cannot reach player helpers.
    The dispatcher resolves arguments, so a runtime parser is re-fed a line
    rebuilt from `literals` + typed arguments.
25. **A content-carrying native package is bigger than a code-only one** (27,717 B
    vs the reference's 7,570 B): `assets/` and `data/` are packed beside `src/`. An
    `.hmod` with both halves is the *point* of the SDK format — the AE2 embedding
    pattern in AGENT_MOD_STRUCTURE §1 is historical.
26. **`cargo horizon-mod check <file> --json` reports `"capabilities":[]` and
    `"side":null`** even for a manifest that declares both (same in the green
    reference); it reports what the *checker* resolved, not the manifest. Do not
    "fix" the manifest for it.

### Verification (this migration)

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

| Check | Command | Result |
|---|---|---|
| `runtime.rs` carried | `cp` + `cmp` + `sha256sum` | byte-identical, `069556ad…03f2`, also inside the package (`unzip -p … src/runtime.rs \| sha256sum`) |
| Runtime unit tests | `CARGO_INCREMENTAL=0 cargo test` | **11 passed**, 0 failed (`evidence/buildinggadgets2/sdk-test.log`) |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| Formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| Package | `rm -f Cargo.lock; cargo horizon-mod build` | `target/horizon-mod/buildinggadgets2.hmod`, exit 0 (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check buildinggadgets2.hmod` | `buildinggadgets2 0.2.0 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)`, **27,717 B**, sha256 `fa3e6ce7…a6d5`, exit 0 (`sdk-check.log`, `sdk-check-json.log`) |
| What it uses | `cargo horizon-mod check buildinggadgets2.hmod --uses` | 227 surface entries, exit 0 (`sdk-uses.json`) |
| Fits the build | `cargo horizon-mod fits buildinggadgets2.hmod --build nightly` | `0b14f8a  made for`, exit 0 (`sdk-fits.log`) |
| Determinism | `rm -f Cargo.lock; cargo horizon-mod build` again, then a **cold** build after `rm -rf target` | same 27,717 B, same sha256 `fa3e6ce7…a6d5` both times |
| Package contents | `unzip -l` | 18 entries: manifest, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `icon.png`, `src/{lib,runtime,support}.rs`, `uses.json`, the 4 `assets/` and 4 `data/` files; **no `Cargo.lock`** |
| Symbol line | `python3 ports/tools/check-skills.py --notes ports/buildinggadgets2` | `Skills:` line present |

### Not re-run in this migration

- **The in-game e2e.** A native mod is compiled into `horizon-client` by the
  launcher, and `horizon-launcher` is not installed in this environment (the
  reference's trap 15, hit again). `tools/e2e.sh` is kept and updated (it now takes
  `GAME=<launcher-built horizon-client>`; the `--dev-mod` and embedded-content legs
  are gone with the format), and it must be run — default, `COUNTERFACTUAL=1` and
  `SELFTEST=1` — before this port claims `ported`. The 0.1.0 transcripts
  (`evidence/buildinggadgets2/e2e*.log`, `check-e2e-logs.py` 4/4) stand as the
  behaviour's evidence: the row maths, the payment rule, the counterfactual and the
  six-block selftest are unchanged, and `runtime.rs` is the same file.
- **The staged ladder** (`stage`/`check`/`test`/`build` through `tools/horizon.py`)
  — those tools are deleted with the retired format.

## Changelog (0.2.0) — supersedes #889

> **Building Gadgets 2 0.2.0** — republished as a native `.hmod` with the SDK
> (mod API 0.2). This supersedes the 0.1.0 `.hnmod` submission (**PR #889**), which
> the registry closed on 2026-11-18 because the `.hnmod` container was retired and
> no longer loads. The package is now one flat file: the building gadget's code
> (`src/`) and its content (`assets/`, `data/`) travel together, so the embedded
> AE2-style content copy of 0.1.0 is gone. Behaviour is unchanged — sneak-click
> selects, a plain right click builds the selected block's horizontal row within
> the 1..15 range, the state lives in the item's `buildinggadgets2:gadget_state`
> component, the mouse wheel steps the range, `/buildinggadgets2 range <n>` still
> works, and one item of the block's kind is paid per placed cell. The pure planner
> is byte-identical, with its 11 tests. Known gaps are unchanged (one mode, no
> preview/animation queue, no energy, no undo/anchor/GUI) plus one measured
> regression: the SDK exposes `canSurvive` only as a hook a mod answers for its own
> blocks, so a cell is no longer pre-checked against it (see `notes.md`).
> Licence: MIT, with the original notice inside the package.

## Review fixes (2026-11-18 review, rebuilt 2026-11-19 at 0.2.1)

Skills: superpowers/using-superpowers, systematic-debugging, test-driven-development, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

**Finding (`src/support.rs:259`).** Every row cell was written through `LevelRef::set_block`, which
checks residency and height only; the game checks spawn protection and the border at the *clicked*
block, so a permitted click could build across a protected boundary.

**Fix.** `runtime::EditRights` + `runtime::may_edit`/`spawn_protected`/`inside_border`/
`spawn_protection_from` are the gate `ServerLevel.mayInteract` applies to a player's own placement;
`LevelWorld::can_place` now asks it **per destination** (build permission, `mayUseItemAt`, the world
border at the cell, and a dedicated server's `spawn-protection` radius around the shared spawn, with
`LEVEL_ADMINS` exempt). A protected cell is skipped like any other invalid cell, so the row stops
short of it instead of crossing it.

**Tests (red before the fix, green after).** `runtime::tests::{an_unprotected_row_cell_is_buildable,
a_player_who_may_not_build_is_refused, a_cell_outside_the_border_is_refused,
a_dedicated_servers_spawn_protection_is_enforced_per_cell, the_spawn_protection_radius_is_the_servers_own}`
— the four refusal tests were run against a stub `may_edit -> true` first and failed there.

**Ladder (0.2.1).** `cargo test` 16 passed · `cargo clippy --all-targets -- -D warnings` exit 0 ·
`cargo fmt --check` exit 0 · `cargo horizon-mod build` twice → identical sha256 · `check` exit 0
(`buildinggadgets2 0.2.1 … for 26.3`) · `fits --build 0b14f8a` → `0b14f8a made for`, exit 0.

**In game.** `launcher-game.sh run buildinggadgets2` → `PASS` + `E2E EXIT:0` (5 blocks placed, 5 stone
spent); `GAME=$CONTROL_GAME` → `FAIL: the package did not provide buildinggadgets2:gadget_building` +
`EXIT:1`. Evidence: `ports/_shared/evidence/buildinggadgets2-e2e/`.

**Package.** `buildinggadgets2.hmod` **31,390 B**, sha256
`46970fcf90e47aaa875e9173f5ecf6529597a5f5885e4bff075aedfe4c3aa4b6`.

## In-game leg repair (2026-11-19, version 0.2.2)

Skills: superpowers/using-superpowers, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

`runtime::may_edit`'s `mayUseItemAt` input was asked at `[0, 0, 0]` instead of the clicked cell; the
engine resolves that position against the level and answers `None` for one it cannot place in, which
refused the placement. Found while the redstonepen leg hit the same latent bug. It now asks at
`event.pos`. No behaviour change in the ladder; the row still places and pays.

**Legs (0.2.2).** modded `PASS` + `EXIT:0` (5 placed, 5 stone spent); control `EXIT:1`.
`check-e2e-logs.py buildinggadgets2-e2e --strict` → 2/2. `cargo test` 16 · clippy 0 · fmt 0 · two
builds byte-identical · `check`/`fits` exit 0.

**Package.** `buildinggadgets2.hmod` **31,461 B**, sha256
`de10c06860205d2d70c7bd93e5901aa1809a298043721be181d2d9079282b3cb`.

## Changelog (0.2.3) — slice 2, the mode family

> **Building Gadgets 2 0.2.3** — the building gadget now has all eight modes
> upstream registers: `build_to_me` (the new default, as upstream), `vertical_wall`,
> `vertical_column`, `surface`, `horizontal_wall`, `stairs`, `horizontal_row` and
> `grid`, each translated from its own `util/modes/*.java` class and pinned by unit
> tests plus an in-game phase asserting the exact cells and the stone consumed.
> `/buildinggadgets2 mode <name>` selects a mode (the radial wheel is a client
> screen, so the command stands in for it) and `/buildinggadgets2 placeontop on|off`
> toggles `PLACE_ON_TOP`; both live in the gadget's `buildinggadgets2:gadget_state`
> component with the range and selection. Grouped modes change with the click's face
> and the player's facing exactly as upstream's collectors do, including upstream's
> centre-cell gate for `horizontal_wall` and its own sign rule for `stairs`. The
> package re-pins to Nightly `0a875da` (mixins `e59da909`). Known gaps are otherwise
> unchanged: no anchor or undo, no preview/animation queue, no energy, no bound
> inventories, no mode wheel or GUI, and the other gadgets are later slices.

## Review repair (0.2.4) — the placement hold and the border's upper edge

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-review

Review: upstream PR **#1576**, reviewer **NikitaMGrimm** at `8b1ad188`, two items. This
version answers both; the first one is a **deliberate behaviour reduction** the reviewer
offered as the alternative when the pin has no cancellable placement path.

### Item 1 — a protection mod cannot cancel a destination (the important one)

The finding was that every generated destination was written with `LevelRef::set_block`
(`src/support.rs`, the old `place`), so a protection mod could not veto an individual
position. The reviewer allowed either an actor-aware, cancellable placement path for each
destination, or holding the area modes. **The symbol check came back negative**, so the
build is held.

**What was checked** — the authoritative SDK at the campaign pin
(`ports/_shared/sdk/0a875da/`: `surface.json` with 749 hooks / 215 traits / handles /
functions, `api-source/crates/**`, and the pin's own `docs/mods.md`,
`docs/native-mods.md`, `docs/modpacks.md`):

| Symbol | Shape | Why it is not the veto point |
|---|---|---|
| `entity_place` hook (`api-source/crates/horizon-mod-vocab/src/hooks/entity_place.rs:1-8`) | **cancellable**, `placer: Option<EntityRef>`, `pos`, `state`, `placed_against`, `placement`; NeoForge's `BlockEvent.EntityPlaceEvent(s)` | This *is* the per-position veto a protection mod implements — but it is fired **only inside `BlockItem.place`** ("heard where `BlockItem.place` has found the position and state and checked survival, before anything is set"). A mod has no host call that fires it for a position it chose. |
| `PlayerRef::place_block` (`api-source/crates/horizon-mod-vocab/src/host/players.rs:1146-1163`) | actor-aware: takes the player's turn, writes with a placement's flags, then `Block.setPlacedBy` | Its contract says *"Nothing else of vanilla's `BlockItem.place` or `useOn` runs: no check that the block may be placed there (survival, entity obstruction, build permission, spawn protection) … The player is only whose turn `setPlacedBy` runs in, so it is not 'as player' in the sense of a use-on."* So `entity_place` never hears it: the same bypass with a player's name on it. |
| `PlayerRef::perform_action` / `perform_action_scoped` (`…/host/players.rs:786-810`) | the **only** call documented to run "reach, protection and the border, then the action", and the pre-hooks (so `entity_place` and a mod's veto do run) | fake-player only: `ActionRefused::NotFake` is documented as *"The handle is a real player's."* (`…/backend.rs:9-14`). A gadget held by a real player cannot use it, and `PlayerActionSetup` acts through "the existing fake" — a mod cannot create one (`PlayerRef::is_fake` / `remove_fake` only). |
| `LevelRef::set_block` (`…/host/levels.rs:915-928`), `world_edit().edit_blocks` | actor-free write / bulk write | what the port did; no permission, no hook, no per-cell answer. |
| `PlayerRef::may_use_item_at` | `Player.mayUseItemAt` for one position | vanilla's own gate (adventure `can_place_on`), already used by `runtime::may_edit`; no third-party veto. |
| hook names matching `place\|permission\|protect\|alter\|lock` in `surface.json` | — | `entity_place` and `on_block_placed_by`/`state_for_placement`/`may_place_on`/`fluid_place_block`/`block_lock_check` are the only near misses; none asks *"may this actor write here"* per position. `block_lock_check` is a guest's block **inventory** lock. |
| `mod_api::dispatch_entity_place` (`…/generated/mod_api_generated.rs:15323`) | the engine's own dispatcher | `#[doc(hidden)]` and **absent from `surface.json`** (0 hits): unstamped plumbing, not a mod-callable path, and the campaign's rule is stamped APIs only. |

**Why holding is a regression, not fidelity.** Upstream's `BuildingUtils.build` asks
`level.mayInteract(player, pos)` *and* fires the NeoForge place event for every cell
(`FINDINGS.md` → "Build and payment"), so a per-cell veto is what the mod means to do; the
pin simply offers no call that fires that event for a position a mod chose. The vanilla
half of that check (`mayInteract`: build permission, `mayUseItemAt`, the border, spawn
protection) is ported per destination in `runtime::may_edit`.

**What the port does instead.** `runtime::placement_hold()` returns
`Some(NO_CANCELLABLE_PLACEMENT)` while that is true, and `runtime::build()` turns a click
into `Attempt::Held { allowed }` instead of `Attempt::Cells`. The glue then writes
nothing, spends nothing, and answers on the action bar:

```
Building Gadget: held (blocked: no-cancellable-placement) - 24 cells would be allowed, 0 placed, 0 spent
```

and `/buildinggadgets2` ends its status line with `, placement held (blocked:
no-cancellable-placement)` so a player learns why before clicking. The **plan still runs**
(it only reads the world), so the reply reports the cells the per-destination gate would
have allowed: that keeps the gate — and the world border, item 2 — observable in game while
the write is held, and it means the eight mode planners stay proven.

What stays live: the item, its recipe/advancement/tab/model/texture/lang, sneak-click
selection into the `buildinggadgets2:gadget_state` component, `/buildinggadgets2
mode|range|placeontop`, the mouse-wheel range step, the status readout and all eight mode
plans. What is held: the block writes, and with them the item payment.

**Lifting the hold** is one line (`placement_hold` → `None`) once the pin can write a
position as the acting player and report a listener's cancellation. The issue shape is
recorded under `### Gaps` below.

### Item 2 — the world border's upper X and Z bounds are exclusive

`runtime::inside_border` compared `x <= max_x && z <= max_z`. The engine's rule is
`BorderGeometry::is_within_bounds(x, z, margin)`
(`horizon/crates/horizon-core/src/geometry/border.rs:240-243`, vanilla
`WorldBorder.isWithinBounds` at zero margin): `x >= min_x - margin && x < max_x + margin`
— **lower edges inclusive, upper edges exclusive**. The port now uses `x >= min_x && x <
max_x && z >= min_z && z < max_z`. `runtime::may_edit`'s "on the edge is allowed"
assertion in `a_cell_outside_the_border_is_refused` had encoded the defect and was
corrected with it.

### Red → green (quoted)

The five new tests and the corrected border assertion were run **first** against the
pre-fix code: `inside_border` still inclusive, `placement_hold()` stubbed to `None` (the
old behaviour, "placement is available"). Verbatim, from
`ports/_shared/evidence/buildinggadgets2/sdk-test-red-0.2.4.log` (`EXIT:101`):

```
---- runtime::tests::a_build_click_is_held_while_no_cancellable_placement_exists stdout ----
thread 'runtime::tests::a_build_click_is_held_while_no_cancellable_placement_exists' panicked at src/runtime.rs:1264:38:
the gadget wrote 24 cells through a raw write while placement is held (blocked: no-cancellable-placement)

---- runtime::tests::a_cell_exactly_on_the_border_upper_bound_is_refused stdout ----
thread 'runtime::tests::a_cell_exactly_on_the_border_upper_bound_is_refused' panicked at src/runtime.rs:1247:9:
exactly on max_x

test result: FAILED. 29 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Green: **35 passed, 0 failed** (`sdk-test.log`). The new tests are
`a_build_click_is_held_while_no_cancellable_placement_exists`,
`a_held_click_says_it_placed_and_spent_nothing`,
`a_held_click_reports_the_cells_the_gate_would_allow`,
`the_border_upper_edge_is_exclusive`,
`a_cell_exactly_on_the_border_upper_bound_is_refused`, plus the corrected
`a_cell_outside_the_border_is_refused` and the two reply tests.

### In game (fail-capable, on the delivered bytes)

`tools/e2e.sh` launches an `--offscreen` survival world, crafts the carried recipe in a
real crafting table (missing-lapis control empty), then runs **9 mode phases and 2 border
phases**. Every phase asserts three things that can each fail: the world scan shows **no
block written**, the stone count is **unchanged**, and the parsed action bar is **exactly**
the held reply for that mode's cell count. The counts are the cell sets the *previous*
version proved by placing them, derived from `util/modes/*.java`, so a mode that plans the
wrong shape fails on the number, not on a generic message.

The "no block written" assertion scans the same box (`-8 147 -8` to `8 166 8`, stone
only, minus the clicked cell) and uses the same filter that **observed** 0.2.3's
placements (5, 24, 9 … cells in `retired-0.2.3/e2e-modes.log`), so it is not a blind
scan; the SELFTEST run additionally requires `[[99,99,99]]` to fail against it.

Quoted from `ports/_shared/evidence/buildinggadgets2-e2e/e2e-hold.log`:

```
== phase build_to_me: mode default, range 5, place-on-top off ==
  placed: []
  consumed: 0 stone
  reply: Building Gadget: held (blocked: no-cancellable-placement) - 3 cells would be allowed, 0 placed, 0 spent
== phase vertical_wall: mode vertical_wall, range 5, place-on-top off ==
  placed: []
  consumed: 0 stone
  reply: Building Gadget: held (blocked: no-cancellable-placement) - 24 cells would be allowed, 0 placed, 0 spent
== border phase edge_at_the_upper_bound: worldborder size 4, upper x edge at 2 ==
  worldborder get: The world border is currently 4 block(s) wide
  placed: []
  consumed: 0 stone
  reply: Building Gadget: held (blocked: no-cancellable-placement) - 19 cells would be allowed, 0 placed, 0 spent
== border phase one_cell_inside_the_bound: worldborder size 6, upper x edge at 3 ==
  worldborder get: The world border is currently 6 block(s) wide
  placed: []
  consumed: 0 stone
  reply: Building Gadget: held (blocked: no-cancellable-placement) - 24 cells would be allowed, 0 placed, 0 spent
PASS: the carried recipe crafted one gadget with its ingredients consumed (a missing lapis matched nothing); all 9 modes held the build with the exact upstream cell count, 0 blocks written and 0 stone spent; and the world border's upper edge refused the cell exactly on it while allowing it one cell inside
EXIT:0
```

**The border differential.** `vertical_wall` at range 5 around the clicked block puts its
cells at `x = -2..2`, `y = 148..152`, `z = 0`. A border centred at `(0,0)` of size 4 ends
at **x = 2**, so that column is *exactly on the upper bound*: 4 columns × 5 rows − the
clicked cell = **19**. Size 6 ends at x = 3, so the same column is *one cell inside* and
allowed: 5 × 5 − 1 = **24**. Under the old inclusive rule both would have been 24 (size 4
would have allowed x = 2) and 24, so the 19 is the fix, measured in game, not inferred.
Both legs print the border size the server reports and the player's position.

**Legs.** modded `PASS` + `EXIT:0`; control (`GAME=$CONTROL_GAME`, the same Nightly without
the package) `FAIL: the carried gadget recipe did not craft…` + `EXIT:1` — the gadget id
does not exist without the package, so the PASS is not vacuous; `SELFTEST=1` inverts every
phase (one block written, one cell too many in the reply) and **all 11 phases reject** it:
`FAIL: SELFTEST: all 11 phases rejected a deliberately wrong expectation (22 assertions)`
+ `EXIT:1`. `check-e2e-logs.py buildinggadgets2-e2e --strict` → **3/3**. The pre-0.2.4
mode logs moved to `ports/_shared/evidence/buildinggadgets2-e2e/retired-0.2.3/`: they
describe a build that wrote blocks, which this version does not.

### Coverage — every family in the jar (0.2.4)

| Family | Status |
|---|---|
| `build_to_me` (default) | plan tested — phase `build_to_me`, 3 cells would be allowed, 0 written |
| `vertical_wall` | plan tested — phase `vertical_wall`, 24 cells would be allowed |
| `vertical_column` | plan tested — phase `vertical_column`, 4 |
| `surface` | plan tested — phase `surface`, 1 |
| `horizontal_wall` | plan tested — phase `horizontal_wall` (range 3), 9 |
| `stairs` | plan tested — phase `stairs`, 4 |
| `horizontal_row` | plan tested — phase `horizontal_row`, 4 |
| `grid` / `place_on_top` | plan tested — phases `grid` (8) and `grid_on_top` (9) |
| the block writes and the item payment | **held** — `blocked(no-cancellable-placement)`; every phase asserts 0 written and 0 spent |
| per-destination placement gate | tested — `may_edit`/`inside_border` units + border phases (upper edge at 19 vs 24) |
| mode/range/placeontop/status replies | tested — replied by value, status names the hold |
| carried recipe | tested — crafted in a table; ingredients consumed; missing-lapis control empty |
| gadget item | tested — crafted, given, selected, held; unknown in the control game |
| anchor, undo, energy | not ported — no anchor/undo list and no FE hook on the gadget |
| preview, per-tick queue, retry | not ported — see the gaps |
| bound inventory / AE2 / Curios | not ported — payment walks the main inventory |
| mode wheel, GUI, tooltip | not ported — the mode is a command; the screen is client work |
| paste/rotate/cut/destruction/exchange gadgets | not ported — later slices |

### Gaps (0.2.4)

The list under `### Gaps (current, 0.2.3)` still holds, with one change and one addition:

- **The block writes are held — `blocked(no-cancellable-placement)`.** A plain right
  click plans its cells and writes none, so no item is paid. This lasts until the pin has
  a placement path that names the acting player, runs vanilla's own placement gate, fires
  the listeners that may veto it, and answers whether one did. The symbols checked are in
  the table above; the two that come closest are `PlayerRef::place_block` (actor-aware, no
  protection) and `PlayerRef::perform_action` (protection and hooks, fake players only).
  **Issue shape:** *"A host call that writes one block as the acting player through
  vanilla's placement path — the `mayInteract` gate, `entity_place` listeners, then
  `setPlacedBy` — and answers `Placed`/`Cancelled`/`Refused`, so a builder can skip one
  cell instead of writing through a protection mod."* It also lifts the upstream-recipe
  limitation below, because the same call runs `BlockItem.place`'s rules.
- **Still no `canSurvive` pre-check** (0.2.3's first gap). It matters less while the write
  is held; it comes back with it.

*Closed by 0.2.5:* the `buildinggadgets2:deny` tag (gap of 0.2.3, repeated in 0.1.0) is carried and
read — see [`## Slice 4`](#slice-4--the-buildinggadgets2deny-tag-2026-11-23-version-025).

*Closed by 0.2.6:* the **whole state string** as the selection (gap of 0.2.3, repeated in 0.2.4/0.2.5)
is cleaned as upstream's `cleanBlockState` does — see
[`## Slice 5`](#slice-5--cleanblockstate-on-the-selection-2026-11-23-version-026).

### Verification (0.2.4)

| Check | Result |
|---|---|
| `cargo test` | **35 passed**, 0 failed (`sdk-test.log`; red first: `sdk-test-red-0.2.4.log`, `EXIT:101`) |
| `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| `cargo fmt --check` | exit 0 (`sdk-fmt.log`) |
| `cargo horizon-mod build` twice, `rm -f Cargo.lock` before each | **40,798 B**, `cmp` identical (`sdk-build.log`, `sdk-build2.log`) |
| `cargo horizon-mod check` | `buildinggadgets2 0.2.4 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)`, exit 0 (`sdk-check.log`) |
| `fits --build 0a875da` | `0a875da  made for` (`sdk-fits.log`) |
| `fits --build 0b14f8a` | `0b14f8a  fits` — a different build, so the pin run is not a tautology (`sdk-fits-other.log`) |
| `verify-sdk-package.py buildinggadgets2` | 1/1 |
| e2e (modded, delivered bytes) | `PASS` + `EXIT:0` (`e2e-hold.log`) |
| e2e (control, same Nightly without the package) | `FAIL: the carried gadget recipe did not craft…` + `EXIT:1` (`e2e-hold-control.log`) |
| e2e (`SELFTEST=1`) | 11/11 phases rejected, `FAIL` + `EXIT:1` (`e2e-hold-selftest.log`) |
| `check-e2e-logs.py buildinggadgets2-e2e --strict` | 3/3 |
| `check-skills.py --notes ports/buildinggadgets2` | `Skills:` line present |

**Package.** `buildinggadgets2.hmod` **40,798 B**, sha256
`927409d7750cc246e1e98630b6355bc9e90ba6c7cdf17c80f9e87bb05066f14a`, **18 entries**, no
packed `Cargo.lock`; declared version **0.2.4**.

### Changelog (0.2.4)

> **Building Gadgets 2 0.2.4** — two review repairs. (1) The gadget no longer writes
> blocks: this pin has no placement call that names the acting player *and* lets a
> protection mod cancel one position (`PlayerRef::place_block` is actor-aware but runs
> none of `BlockItem.place`'s protection; `PlayerRef::perform_action` is fake-player only;
> the cancellable `entity_place` hook only fires inside `BlockItem.place`), so a plain
> right click places nothing, spends nothing and says `blocked:
> no-cancellable-placement` with the number of cells the player's own gate would have
> allowed. Selection, all eight mode plans, the range/mode/place-on-top commands and the
> status readout keep working, and the hold is one constant away from lifting. (2) The
> world border's upper X/Z edges are now exclusive, as the engine's
> `BorderGeometry::is_within_bounds` is, so the cell exactly on the bound is refused and
> the one cell inside is allowed (proved in game: 19 vs 24 cells). Known gaps are
> otherwise unchanged: no anchor or undo, no preview/animation queue, no energy, no bound
> inventories, no mode wheel or GUI, and the other gadgets are later slices.

## Slice 3 — the held-cell preview: measured, then parked (2026-11-23)

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

**The slice.** The gap it comes from is `### Gaps (current, 0.2.3)`: "**Placement is immediate**: no
preview, no per-tick queue, no retry pass for supports placed later, no `BlockItem.place`/
`setPlacedBy`, block-entity data of the placed state is not carried, no sound or particles." The
preview is the half that needs no placement call at all — it draws the cells `runtime::plan` already
produces, so it is the player-visible explanation of the 0.2.4 hold, and the tier fits (`deep` →
renderers).

**Outcome: parked, nothing shipped. 0.2.4 stays the published version** (`buildinggadgets2.hmod`
40,798 B, sha256 `927409d7…f14a`, and the tree was reverted to exactly that). The package, its
archive and every log under `../_shared/evidence/buildinggadgets2*/` are unchanged; the
measurements and the two frames are in
[`../_shared/evidence/buildinggadgets2-preview-parked/`](../_shared/evidence/buildinggadgets2-preview-parked/README.md).

### Surface measured at the pin (`ports/_shared/sdk/0a875da/`)

| Symbol | Shape |
|---|---|
| `render_level_stage` | hook v1 · `side = "client"` · `contract = "listen"` · non-cancellable · args `stage: RenderLevelStage`, `view: &WorldRenderView`, `draw: &'a mut dyn WorldDrawContext` |
| `RenderLevelStage` | 5 variants, `#[non_exhaustive]`: `AfterSky`, `AfterOpaqueBlocks`, `AfterTranslucentBlocks`, `AfterWeather`, `AfterLevel` |
| `WorldDrawContext` | `draw_instance_group`, **`draw_model_instances(ModelRef, &[ModelInstance]) -> Result<(), InstanceError>`**, `lines`, `quads` (unlit, alpha-blended, white atlas texel), `pipeline_quads`, `usage` |
| `draw_model_instances` | one batch per model; 16,384 copies per view and per call; `InstanceError::{Model, NonFinite, InvalidLight, CapacityExceeded, Translucent}` — "a model with a translucent face is refused and never drawn as solid" |
| `ModelInstance` | `origin: [f64; 3]` world blocks, `transform: [f32; 12]` (`IDENTITY_TRANSFORM`), `tint: 0xAARRGGBB` ("alpha multiplies only faces that have a tint index"), `light: LightCoords`, `overlay: Overlay` |
| how a `ModelRef` is obtained | `standalone_models` (collect v2: `models`, `forms`) → `model_baking_completed` → `host.render_assets().model(&Identifier)` |
| **a model without carrying a texture** | **yes** — a model file naming `minecraft:block/white_concrete` with `tintindex: 0` on all six faces; no texture bytes in the package, none regenerated |

### What was measured in game, in order

The pure side came first and was green (39 tests, the four new ones red first at 36 passed / 3
failed, `EXIT:101`); the glue then needed four diagnostic builds, each a `--offscreen` frame:

1. the hook fires: a `WorldDrawContext::quads` panel draws at `AfterTranslucentBlocks`;
2. the model route works: the `standalone_models` id resolves and `draw_model_instances` draws a
   green cube with an `0xFF00FF00` tint — so the tint and the vanilla-texture model are right;
3. **the render hook's host carries no player**: `local_player()` answers `None` there, so the plan
   cannot be made in `render_level_stage` (it was moved to `client_tick_post`, which does have the
   full client host);
4. **the client has no exact held-stack read**: `player_inventory_slots` is *Side: server — "None …
   on a client"* (measured `None`); `player_inventory` returns the stacks **without** slot numbers,
   so the selected hotbar slot cannot be resolved; `render_hand`'s `HandRender::stack` is the only
   client-side stack copy and it read back **without** the `buildinggadgets2:gadget_state`
   component (`held=true`, no selection) while the client's own inventory copy of the same stack
   carries `selected:"minecraft:stone"`.

**Blocker:** `blocked(no-client-held-stack-components)` — a client-side read of the main-hand stack
*with its item components* is what the preview needs and what the pin does not stamp.
**Issue shape:** *"A client-side read of the selected hotbar stack, or `player_inventory` returned
with its slot numbers, so a client renderer can read the item components the server wrote (here the
gadget's selection and mode) without a server round trip."*

Not separable inside this slice's budget: whether `HandRender::stack` really drops the component or
the probe's sneak click (made at an extended `block_interaction_range`) was refused by the server, so
that no selection existed yet. Separating them needs one more package build plus an e2e slot; the
wave's e2e queue is the bottleneck, so the slice stops rather than shipping a preview whose pixels
were never observed. The two frames and the exact readings are in
`../_shared/evidence/buildinggadgets2-preview-parked/README.md`.

### What was reverted

`src/runtime.rs` (the `Ghost`/`preview`/`ghost` API and its four tests), `src/support.rs` (the
preview glue: the tick plan, the render draw, the standalone-model registration, the hand cache),
`src/lib.rs` (the four hooks), `assets/buildinggadgets2/models/block/preview_cell.json`,
`tools/preview_pixel.py`, the two `preview_phase`s in `tools/e2e.sh`, and the version bump
(`horizon-mod.toml`/`Cargo.toml` back to 0.2.4). The rebuilt archive is byte-identical to the
published 0.2.4 (`cargo horizon-mod build` → 40,798 B, `927409d7…f14a`), which is what
`verify-sdk-package.py buildinggadgets2` checks against the tree.

## Slice 4 — the `buildinggadgets2:deny` tag (2026-11-23, version 0.2.5)

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

**Gap it closes.** `### Gaps (current, 0.2.3)`: *"The `buildinggadgets2:deny` tag is not consulted (no tag
read is wired)"* — the last family whose absence is visible through the held reply, because the plan
runs while the write is held and the reply reports the cells the gate would allow.

### The jar's tag, enumerated before any code

`unzip -p buildinggadgets2-1.3.9.jar data/buildinggadgets2/tags/block/deny.json` — 210 B,
`sha256 227775ae7b654825003e0626823ebcb89c6a95f091202a791c60b4c3a56c1449`, carried **verbatim** to
`data/buildinggadgets2/tags/block/deny.json` (`unzip -p` + `cmp`, byte-identical):

| entry | what it resolves to in 26.3 (`block-tags.json`) | blocks |
|---|---|---:|
| `minecraft:piston_head` | itself | 1 |
| `minecraft:bedrock` | itself | 1 |
| `minecraft:end_portal_frame` | itself | 1 |
| `minecraft:candle_cake` | itself | 1 |
| `#minecraft:beds` | the 16 coloured beds (no nested tags) | 16 |
| `#minecraft:portals` | `nether_portal`, `end_portal`, `end_gateway` | 3 |
| `#minecraft:doors` | 22 doors (`wooden_doors` 11 + iron + 10 copper) | 22 |
| **total** | | **45** |

Every nested tag resolves in the prepared 26.3 resources, so the carried file is not dead data.

### Where upstream consults it (and where it does not)

- `GadgetUtils.isValidBlockState` (`GadgetUtils.java:36-40`): `blockState.is(BG2BlockTags.BG2DENY)`
  → false, beside `getDestroySpeed < 0` and a non-source fluid.
- Its callers on the **building** gadget: `GadgetBuilding.onShiftAction` — the **selection** — which
  answers `buildinggadgets2.messages.invalidblock` = **"Invalid Block"** on the action bar and leaves
  the gadget as it was. `BuildingUtils.build` (the destination loop) checks `mayInteract`, the place
  event, `canBeReplaced`, `canSurvive` and the items, **not** the tag.
- The tag's other callers (`BaseMode.isExchangingValid`, `BuildingUtils.exchange`,
  `BuildingUtils.removeTickHandler`) are the exchange and destruction gadgets, which are later slices:
  there the tag gates the *existing block at a destination*.

### What shipped

1. **The tag file**, verbatim (above), inside the package.
2. **`runtime`** gains the whole of `isValidBlockState` as a pure rule, so the glue no longer holds
   business logic: `DENY_TAG`, `denied(Option<bool>)`, `FluidAt`, and
   `selectable(in_deny_tag, destroy_speed, fluid)` = *not denied* **and** *a readable, non-negative
   destroy speed* **and** *(empty fluid or a source)*. `denied` is `Some(true)` only: an unknown
   block/tag (`Some(false)`) and a tag table that has not loaded (`None`) are not refusals, per
   `TagsHost::block_in_tag`'s own contract.
3. **`support`**: one stamped, both-side read — `host.block_in_tag(state.block(), runtime::DENY_TAG)`
   (`TagsHost`, `surface.json` `method:prelude::TagsHost::block_in_tag`, hash `4f53aa81…`) — on the
   selection path (upstream's consumer, now with the `Invalid Block` reply) and as a **destination**
   refusal in `can_place`.

### The measurement that chose the in-game proof

Every one of the 45 resolved members is **non-replaceable in every state** in 26.3
(`gameplay-facts.json`, per state: 0 replaceable states over all 45 × their states). So a
destination refusal is, with the *carried* tag, already implied by `can_place`'s replaceable test,
and a count difference *from the carried tag alone* cannot be observed — a run that expected one
would not be able to fail. The destination read is therefore proved where a tag member **is**
replaceable, which is also how tags are meant to be used: a world data pack that (re-)declares
`buildinggadgets2:deny` with `minecraft:snow` (a replaceable block). The engine takes a pack's file
over a mod's contributed file for the same path (`overlay_files`), so in the test world the tag names
snow; `/reload` rebuilds the world's tables (`Game::reload_world_tables`), which is what
`TagsUpdatedEvent` does upstream.

### Where the port departs from the Java (recorded, not silent)

The **destination** refusal is the exchange gadget's rule applied to the build path, which the Java
building gadget does not do. With the carried tag it is invisible (the measurement above); it decides
only for a tag a data pack extends with a replaceable block, where the alternative is a gadget that
refuses to *select* a block and then *overwrites* it. Recorded as a deviation for the reviewer; it is
the only way the brief's destination rule has an observable effect at this pin.

### Red → green (unit)

`ports/_shared/evidence/buildinggadgets2/sdk-test-red-0.2.5-deny.log` (`EXIT:101`): the six new tests
run against `denied()` stubbed to `false` ("no tag read is wired", the 0.2.4 behaviour) —
**38 passed, 3 failed**:

```
---- runtime::tests::the_deny_tag_refuses_only_a_positive_answer stdout ----
thread 'runtime::tests::the_deny_tag_refuses_only_a_positive_answer' panicked at src/runtime.rs:1412:9:
a tagged block is denied

---- runtime::tests::a_tagged_block_cannot_be_selected stdout ----
thread 'runtime::tests::a_tagged_block_cannot_be_selected' panicked at src/runtime.rs:1422:9:
a block in buildinggadgets2:deny must not be selectable

---- runtime::tests::a_tagged_destination_is_refused stdout ----
thread 'runtime::tests::a_tagged_destination_is_refused' panicked at src/runtime.rs:1469:9:
a denied destination

test result: FAILED. 38 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Green: **41 passed, 0 failed** (`sdk-test.log`). The six new tests are
`the_deny_tag_refuses_only_a_positive_answer`, `a_tagged_block_cannot_be_selected`,
`an_untagged_block_with_a_positive_destroy_speed_is_selectable`,
`an_unbreakable_block_cannot_be_selected`, `only_a_source_fluid_is_selectable`,
`a_tagged_destination_is_refused`.

### Red → green (in game, against the read switched off)

The tagged count's assertion was also run **red**: `src/support.rs`'s destination read was replaced
by a stub (the selection read kept), the manifest bumped to 0.2.6 so the launcher could not reuse a
cached build (`Compiling buildinggadgets2 (1 of 1)`), and the e2e run as delivered
(`ports/_shared/evidence/buildinggadgets2-e2e/e2e-deny-red-destination.log`, `EXIT:1`):

```
== deny tag phase deny_destination: snow at 2 of the grid's 8 destinations, 6 would be allowed ==
  destinations (6,150,6) and (-6,150,-6): minecraft:snow, minecraft:snow
  reply: Building Gadget: held (blocked: no-cancellable-placement) - 8 cells would be allowed, 0 placed, 0 spent
FAIL: deny_destination: the held reply was 'Building Gadget: held (blocked: no-cancellable-placement) - 8 cells would be allowed, 0 placed, 0 spent', expected '... 6 cells would be allowed, ...'
```

The same run's `deny_selection` and `deny_destination_control` phases passed, so the red names the
destination read exactly. The tree, the manifest and the package were restored to 0.2.5
(`src/support.rs` and `horizon-mod.toml` sha256 equal the package's copies) and the launcher game
rebuilt from that tree (the exported package's `src/support.rs` is `26dcb8ae…` in both).

### In game (fail-capable, on the delivered bytes)

Quoted from `ports/_shared/evidence/buildinggadgets2-e2e/e2e-deny.log`:

```
== deny tag phase deny_selection: a minecraft:piston_head cannot be selected, stone can ==
  deny-tagged block at 2 151 4: minecraft:piston_head
  selection reply (stone): Selected minecraft:stone
  selection reply (minecraft:piston_head): Invalid Block
  status: Building Gadget: mode grid, range 5, block minecraft:stone, place-on-top off, placement held (blocked: no-cancellable-placement)
  placed: []
  consumed: 0 stone
  reply: Building Gadget: held (blocked: no-cancellable-placement) - 8 cells would be allowed, 0 placed, 0 spent
== deny tag phase deny_destination_control: snow at 2 of the grid's 8 destinations, 8 would be allowed ==
  destinations (6,150,6) and (-6,150,-6): minecraft:snow, minecraft:snow
  reply: Building Gadget: held (blocked: no-cancellable-placement) - 8 cells would be allowed, 0 placed, 0 spent
== the test world declares minecraft:snow in the deny tag, then /reload ==
== deny tag phase deny_destination: snow at 2 of the grid's 8 destinations, 6 would be allowed ==
  destinations (6,150,6) and (-6,150,-6): minecraft:snow, minecraft:snow
  reply: Building Gadget: held (blocked: no-cancellable-placement) - 6 cells would be allowed, 0 placed, 0 spent
PASS: … the carried buildinggadgets2:deny tag refused a sneak click on minecraft:piston_head (leaving minecraft:stone selected) while its destination rule counted 8 cells with ordinary snow and 6 once the test world named snow in the tag
EXIT:0
```

`minecraft:piston_head` is the tagged block because it is a member whose *other* clauses all pass
(destroy speed 1.5, block item air, no fluid), so only the tag can refuse it; it is placed
`facing=west` because that facing's outline is a full 1×1 plate at the west face
(`[0,0,0 … 0.25,1,1]`), so the crosshair actually reaches it.

**The pair.** Same world, same click, only the merged tag differs: 8 with snow untagged, 6 with snow
tagged. A read that always answers `false` reports 8 in the tagged phase (measured in the red run
above); one that always answers `true` reports 0 in the control phase and in all 9 mode phases. The
SELFTEST run inverts every phase's expectation and **all 14 phases reject** (`e2e-deny-selftest.log`,
`FAIL` + `EXIT:1`, 30 assertions).

### Coverage — every family in the jar (0.2.5)

Replaces the 0.2.4 table; unchanged rows are the same verdicts.

| Family | Status |
|---|---|
| `build_to_me` (default), `vertical_wall`, `vertical_column`, `surface`, `horizontal_wall`, `stairs`, `horizontal_row`, `grid`, `place_on_top` | plan tested — phases as in 0.2.4, 0 written, 0 spent |
| `buildinggadgets2:deny` — **selection** | tested — phase `deny_selection`: `Invalid Block`, `minecraft:stone` still selected, held count 8 unchanged |
| `buildinggadgets2:deny` — **destination** | tested — phases `deny_destination_control` (8) and `deny_destination` (6) in one run, 8 vs 6 |
| the block writes and the item payment | **held** — `blocked(no-cancellable-placement)`; every phase asserts 0 written and 0 spent |
| per-destination placement gate | tested — `may_edit`/`inside_border` units + border phases (19 vs 24) |
| mode/range/placeontop/status replies | tested — replied by value, status names the hold |
| carried recipe | tested — crafted in a table; ingredients consumed; missing-lapis control empty |
| gadget item | tested — crafted, given, selected, held; unknown in the control game |
| anchor, undo, energy | not ported — no anchor/undo list and no FE hook on the gadget |
| preview, per-tick queue, retry | not ported — the preview is `blocked(no-client-held-stack-components)` (parked slice, see above) |
| bound inventory / AE2 / Curios | not ported — payment walks the main inventory |
| mode wheel, GUI, tooltip | not ported — the mode is a command; the screen is client work |
| paste/rotate/cut/destruction/exchange gadgets | not ported — later slices (the exchange gadget is the tag's other consumer) |

### Verification (0.2.5)

| Check | Result |
|---|---|
| `cargo test` | **41 passed**, 0 failed (`sdk-test.log`; red first: `sdk-test-red-0.2.5-deny.log`, `EXIT:101`) |
| `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| `cargo fmt --check` | exit 0 (`sdk-fmt.log`) |
| `cargo horizon-mod build` twice, `rm -f Cargo.lock` before each | **43,350 B**, `cmp` identical (`sdk-build.log`, `sdk-build2.log`) |
| `cargo horizon-mod check` | `buildinggadgets2 0.2.5 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)`, exit 0 (`sdk-check.log`) |
| `fits --build 0a875da…` | `0a875da  made for` (`sdk-fits.log`) |
| `fits --build 0b14f8a…` | `0b14f8a  fits` — a different build, so the pin run is not a tautology (`sdk-fits-other.log`) |
| `verify-sdk-package.py buildinggadgets2` | 1/1 |
| `check-recipe-types.py buildinggadgets2` | `buildinggadgets2: ok`, exit 0 |
| e2e (modded, delivered bytes) | `PASS` + `EXIT:0`, 14 phases (`e2e-deny.log`) |
| e2e (control, same Nightly without the package) | `FAIL: the carried gadget recipe did not craft…` + `EXIT:1` (`e2e-deny-control.log`) |
| e2e (`SELFTEST=1`) | 14/14 phases rejected, `FAIL` + `EXIT:1` (`e2e-deny-selftest.log`) |
| e2e (red: destination read switched off) | `FAIL: deny_destination … 8 cells …, expected 6` + `EXIT:1` (`e2e-deny-red-destination.log`) |
| `check-e2e-logs.py buildinggadgets2-e2e --strict` | 7/7 |
| `check-skills.py --notes ports/buildinggadgets2` | `Skills:` line present |

**Package.** `buildinggadgets2.hmod` **43,350 B**, sha256
`9d19a3410c5dea90b062fb5818fea876710085ef412f9a23de151658c9bbfcf7`, **19 entries** (the 18 of 0.2.4
plus `data/buildinggadgets2/tags/block/deny.json`), no packed `Cargo.lock`; declared version **0.2.5**.

### Changelog (0.2.5)

> **Building Gadgets 2 0.2.5** — the mod's own `buildinggadgets2:deny` block tag is now carried and
> read. A sneak click on any of its 45 blocks (the piston head, bedrock, the end-portal frame, candle
> cake, the 16 beds, the 3 portals and the 22 doors) answers upstream's *Invalid Block* and leaves the
> gadget as it was, and a build destination whose block the tag names is refused (that half is the
> exchange gadget's own rule in Java; with the carried tag it agrees with the replaceable test, and it
> decides for a tag a data pack extends — proved in game, 8 cells with ordinary snow at two grid
> destinations and 6 once the tag names snow). The selection rules (`isValidBlockState`) moved into the
> pure, unit-tested planner. The build stays held (`blocked: no-cancellable-placement`), the preview
> stays `blocked(no-client-held-stack-components)`, and the other gaps are unchanged.

## Slice 5 — `cleanBlockState` on the selection (2026-11-23, version 0.2.6)

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

**Gap it closes.** `### Gaps (current, 0.2.3)`: "*Selection stores the whole state string*, so transient
properties (age, lit, waterlogged, open, …) are not cleaned as upstream's `cleanBlockState` does;
orientation properties are preserved." It is the selection path only — no placement call — so the
slice does not touch the 0.2.4 hold.

**What upstream does.** `GadgetBuilding.onShiftAction` → `GadgetUtils.setBlockState` →
`GadgetUtils.cleanBlockState` (`GadgetUtils.java:109-118`): start from
`sourceState.getBlock().defaultBlockState()` and apply every property of the clicked state **except**
those in `DENY_PROPERTIES` (`GadgetUtils.java:53-58`). The list, read from the jar verbatim, is

| Property | Values | What it is |
|---|---:|---|
| `age` (8 instances) | 2, 3, 4, 5, 6, 8, 16, 26 | `AGE_1`, `AGE_2`, `AGE_3`, `AGE_4`, `AGE_5`, `AGE_7`, `AGE_15`, `AGE_25` |
| `half` | 2 | `DoublePlantBlock.HALF` (lower/upper, and the stair/slab bottom/top) |
| `waterlogged` | 2 | `WATERLOGGED` |
| `lit` | 2 | `LIT` |
| `has_record` | 2 | `HAS_RECORD` |
| `has_book` | 2 | `HAS_BOOK` |
| `open` | 2 | `OPEN` |
| `stage` | 2 | `STAGE` |

Everything else is carried over: `facing`, `axis`, `hinge`, `powered`, `shape`, `type`, `rotation`,
`half`-of-a-stair apart, `snowy`, `north/east/south/west` (a fence's connections), … Upstream compares
property **instances**, and `age` is eight of them; `BlockStateProperties` has no `AGE_6`, so a
mod-declared seven-valued `age` survives. The port carries the same identity as (name, value count),
which is exact for every single-instance entry (all two-valued) and for the eight `age` instances.

**What shipped.**

1. **`runtime`** now holds the whole rule, pure: `PropertyFacts` (name + value count),
   `denied_property(name, values)`, `parse_state`, and `clean_block_state(source, default,
   definition)`, which starts from the default state's properties, drops the denied ones, carries the
   rest and serializes in name order. A property the definition does not name is **kept**, so a host
   without metadata tables stores the state as 0.2.5 did instead of erasing its orientation.
2. **`support`**: `stored_state` reads the game's own metadata — `BlockStates::definition`
   (`handle:BlockStates::definition`, stamped), `block_state_default` (`Block.defaultBlockState`,
   `prelude::BlockStatesHost::block_state_default`, stamped), `block_state_string`/`block_state_id` —
   so the sneak click stores `clean_block_state`'s result in the game's canonical form. A fact the
   host cannot answer falls back to the raw state.
3. **The readout**: `/buildinggadgets2` now prints the **stored state string** (`block
   minecraft:furnace[facing=west,lit=false]`) instead of the block id alone, so the cleaning is
   observable in game. A block with no properties still reads as its id (`block minecraft:stone`), so
   every earlier reply assertion is unchanged.

**Red → green (unit).** The five new tests were run first against 0.2.5's behaviour stubbed in
(`denied_property -> false`, `clean_block_state -> Some(source)`), from
`ports/_shared/evidence/buildinggadgets2/sdk-test-red-0.2.6-clean.log` (`EXIT:101`), **42 passed, 5
failed**:

```
---- runtime::tests::cleaning_a_door_drops_half_and_open_and_keeps_facing_and_hinge stdout ----
thread 'runtime::tests::cleaning_a_door_drops_half_and_open_and_keeps_facing_and_hinge' panicked at src/runtime.rs:1567:9:
assertion `left == right` failed: half=upper and open=true are dropped; facing, hinge and powered survive
  left: Some("minecraft:oak_door[facing=east,half=upper,hinge=right,open=true,powered=true]")
 right: Some("minecraft:oak_door[facing=east,half=lower,hinge=right,open=false,powered=true]")

---- runtime::tests::the_deny_list_is_upstreams_property_instances stdout ----
thread 'runtime::tests::the_deny_list_is_upstreams_property_instances' panicked at src/runtime.rs:1516:13:
age with 2 values is one of upstream's AGE instances

test result: FAILED. 42 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Green: **47 passed, 0 failed** (`sdk-test.log`). The six new tests are
`the_deny_list_is_upstreams_property_instances`,
`cleaning_a_door_drops_half_and_open_and_keeps_facing_and_hinge` (the gap line's door: `half=upper`
and `open=true` dropped, `facing`, `hinge` and `powered` carried),
`cleaning_a_crop_resets_its_age_and_keeps_a_mods_own` (a wheat's `AGE_7` reset; a seven-valued
`age` kept), `cleaning_keeps_orientation_and_shape` (stairs: `facing`/`shape` kept, `half`/
`waterlogged` cleaned; stone has no properties), `cleaning_refuses_malformed_text_and_keeps_everything_without_metadata`
and `the_status_reply_names_the_stored_state_string`.

### In game (fail-capable, on the delivered bytes)

Two phases in `tools/e2e.sh`, quoted from `ports/_shared/evidence/buildinggadgets2-e2e/e2e-clean.log`:

```
== clean phase clean_selection: a lit furnace's lit=true is dropped, facing=west survives ==
  furnace at 2 151 4: minecraft:furnace[facing=west,lit=true]
  selection reply: Selected minecraft:furnace
  status: Building Gadget: mode grid, range 5, block minecraft:furnace[facing=west,lit=false], place-on-top off, placement held (blocked: no-cancellable-placement)
== clean phase clean_transient: a ripe wheat's age=7 is transient, the default age=0 is stored ==
  wheat at 2 151 4: minecraft:wheat[age=7]
  selection reply: Selected minecraft:wheat
  status: Building Gadget: mode grid, range 5, block minecraft:wheat[age=0], place-on-top off, placement held (blocked: no-cancellable-placement)
EXIT:0
```

Each phase first **reads and asserts the state it is about to click** (`lit=true`, `facing=west`;
`age=7`), so the expectation cannot hold vacuously on a block that never carried the transient
value. `SELFTEST=1` expects 0.2.5's raw state there and every one of the **16 phases** rejects it
(`32 assertions`, `FAIL` + `EXIT:1`, `e2e-clean-selftest.log`); the control game without the package
fails at the recipe (`EXIT:1`, `e2e-clean-control.log`).

**Why the in-game block is a furnace and not the gap line's door.** The carried
`buildinggadgets2:deny` tag names `#minecraft:doors` (0.2.5), so since 0.2.5 a door cannot be
selected at all — the selection answers `Invalid Block`. A world data pack can only *add* to a tag:
the probe run (`ports/_shared/evidence/buildinggadgets2-e2e/probe-door.sh`, quoted in
`probe-door.log`) re-declared the tag as `{"values":[]}`, reloaded, and the doors were still refused,
so the door can never reach
`cleanBlockState` through a sneak click. Its own cleaning is pinned by the unit test named above, and
the game assertion uses a full cube the tag lets through (`lit` denied, `facing` kept — one denied
and one kept property, both off their defaults). An open door's leaf is also a thin plate: the probe
measured that with `facing=west` it stands at the block's south edge, so only a click from the north
reaches it — recorded here so a later slice does not re-derive it.

### Coverage — every family in the jar (0.2.6)

Replaces the 0.2.5 table; unchanged rows are the same verdicts.

| Family | Status |
|---|---|
| `build_to_me` (default), `vertical_wall`, `vertical_column`, `surface`, `horizontal_wall`, `stairs`, `horizontal_row`, `grid`, `place_on_top` | plan tested — phases as in 0.2.4, 0 written, 0 spent |
| `cleanBlockState` — a denied property | tested — phase `clean_selection` (`lit=true` → `lit=false`), `clean_transient` (`age=7` → `age=0`); unit `cleaning_a_crop_resets_its_age_and_keeps_a_mods_own` |
| `cleanBlockState` — a kept property | tested — `facing=west` survives in `clean_selection`; `facing`/`hinge`/`powered` in the door unit test; `shape` in the stairs unit test |
| `cleanBlockState` — `age` instances | unit tested — the eight arities denied, a seven-valued `age` kept |
| the status readout | tested — `/buildinggadgets2` names the stored state string; every earlier reply still asserted by value |
| `buildinggadgets2:deny` — **selection** | tested — phase `deny_selection`: `Invalid Block`, `minecraft:stone` still selected, held count 8 unchanged |
| `buildinggadgets2:deny` — **destination** | tested — phases `deny_destination_control` (8) and `deny_destination` (6) in one run |
| the block writes and the item payment | **held** — `blocked(no-cancellable-placement)`; every phase asserts 0 written and 0 spent |
| per-destination placement gate | tested — `may_edit`/`inside_border` units + border phases (19 vs 24) |
| mode/range/placeontop/status replies | tested — replied by value, status names the hold |
| carried recipe | tested — crafted in a table; ingredients consumed; missing-lapis control empty |
| gadget item | tested — crafted, given, selected, held; unknown in the control game |
| anchor, undo, energy | not ported — no anchor/undo list and no FE hook on the gadget |
| preview, per-tick queue, retry | not ported — the preview is `blocked(no-client-held-stack-components)` (parked slice, see above) |
| bound inventory / AE2 / Curios | not ported — payment walks the main inventory |
| mode wheel, GUI, tooltip | not ported — the mode is a command; the screen is client work |
| paste/rotate/cut/destruction/exchange gadgets | not ported — later slices (the exchange gadget is the tag's other consumer) |

### Gaps (0.2.6)

The list under `### Gaps (current, 0.2.3)` holds with one closure and no additions: the whole-state
selection is cleaned (above). The block writes stay held (`blocked: no-cancellable-placement`) and the
preview stays `blocked(no-client-held-stack-components)`; the remaining families are unchanged (no
anchor/undo/energy, no bound inventories, no GUI or mode wheel, no preview queue and no other gadget).

### Verification (0.2.6)

| Check | Result |
|---|---|
| `cargo test` | **47 passed**, 0 failed (`sdk-test.log`; red first: `sdk-test-red-0.2.6-clean.log`, 42/5, `EXIT:101`) |
| `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| `cargo fmt --check` | exit 0 (`sdk-fmt.log`) |
| `cargo horizon-mod build` twice, `rm -f Cargo.lock` before each | **48,042 B**, `cmp` identical (`sdk-build.log`, `sdk-build2.log`) |
| `cargo horizon-mod check` | `buildinggadgets2 0.2.6 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)`, exit 0 (`sdk-check.log`) |
| `fits --build 0a875da…` | `0a875da  made for` (`sdk-fits.log`) |
| `fits --build 0b14f8a…` | `0b14f8a  fits` — a different build, so the pin run is not a tautology (`sdk-fits-other.log`) |
| `verify-sdk-package.py buildinggadgets2` | 1/1 (`package matches the tree`) |
| `check-recipe-types.py buildinggadgets2` | `buildinggadgets2: ok`, exit 0 |
| e2e (modded, delivered bytes) | `PASS` + `EXIT:0`, 16 phases (`e2e-clean.log`) |
| e2e (control, same Nightly without the package) | `FAIL: the carried gadget recipe did not craft…` + `EXIT:1` (`e2e-clean-control.log`) |
| e2e (`SELFTEST=1`) | 16/16 phases rejected, `FAIL` + `EXIT:1`, 32 assertions (`e2e-clean-selftest.log`) |
| `check-e2e-logs.py buildinggadgets2-e2e --strict` | **10/10** (the 0.2.5 deny-tag legs still green) |
| `check-skills.py --notes ports/buildinggadgets2` | `Skills:` line present |

**Package.** `buildinggadgets2.hmod` **48,042 B**, sha256
`6885766f687e20941f3538a5576bfdf3a31ee5e97bcc41f18c15c23b08d0c857`, **19 entries** (the deny tag
included), no packed `Cargo.lock`; declared version **0.2.6**; `src/runtime.rs` sha256
`0c2de1593d8b…` in the tree and in the archive. The game leg was rebuilt from this tree after the
docs were written (`Compiling buildinggadgets2 (1 of 1)`), so the bytes under test are the bytes
delivered.

### Changelog (0.2.6)

> **Building Gadgets 2 0.2.6** — a sneak click now stores upstream's *cleaned* block state
> (`GadgetUtils.cleanBlockState`): the transient properties upstream lists (a crop's `age` — all eight
> `AGE_*` instances, a door's or stair's `half`, `open`, `waterlogged`, `lit`, `has_record`,
> `has_book`, `stage`) are stored at the block's own default, while the orientation ones (`facing`,
> `axis`, `hinge`, `powered`, `shape`, `type`, …) are carried over, and `/buildinggadgets2` names the
> stored state string so you can see it. Proved in game: a lit furnace is stored as
> `minecraft:furnace[facing=west,lit=false]` and a ripe wheat as `minecraft:wheat[age=0]`, each with
> the clicked state asserted first, and 0.2.5's raw state is the inverted expectation every phase
> rejects. The build stays held (`blocked: no-cancellable-placement`), the preview stays
> `blocked(no-client-held-stack-components)`, and the other gaps are unchanged.

## Slice 6 — the destruction gadget's destination plan (2026-11-24, version 0.2.7)

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

**Gap it closes.** `### Coverage — every family in the jar (0.2.6)`:
*"paste/rotate/cut/destruction/exchange gadgets | not ported — later slices (the exchange gadget is the tag's other consumer)"*.
Destruction is picked because the deny tag (0.2.5) is its other documented consumer and the
0.2.4 hold lets it land as a **plan + readout only**: a new item, a `gadget_state` field and a
pure planner, with no block removal. Class content, tier deep; no write hook is called.

### The four refusal rules, and what the pin can actually query

From `GadgetUtils.isValidDestroyBlockState` (`:56-61`) plus the campaign's deny-tag clause:

| Rule | Pin query | Verdict |
|---|---|---|
| air refused | `BlockMetadata::name(state.block())` -> `minecraft:air`/`cave_air`/`void_air` (stamped, same read `selectable` uses) | **carried** |
| `destroy_speed < 0` refused | `BlockStates::destroy_speed(state)` -> `Backend::block_state_destroy_speed` #279 (stamped; 26.3 reports bedrock -1, stone 1.5) | **carried** |
| deny-tag member refused | `Host::block_in_tag(state.block(), "buildinggadgets2:deny")` #69 (stamped; the merged tag, so data packs count) | **carried** |
| `RenderBlock` refused | no stamped block-class / render-block surface exists at `0a875da`, and this port places no render block (the build is held). No query was invented. | **`blocked(no-render-block-surface)`** for this line only |

The three queryable lines are the whole `destroyable` predicate (`runtime.rs`); the fourth is
recorded, not faked. Upstream's `getDestructionArea` also consults `mayInteract` and a
`BreakEvent`; those remain under the building gadget's hold (`no-cancellable-placement`).

### What shipped

1. **`runtime`:** `DESTROY_GADGET`, `NO_RENDER_BLOCK_SURFACE`, `DestroyFacts`, `DestroyWorld`,
   `destroyable` (three carried clauses), `destroy_plan` (one y-layer, ±range on x and z,
   keeping only destroyable cells), `destroy_reply` and `destroy_status_reply`.
2. **`support`:** `LevelDestroyWorld` reads the three facts through the stamped host calls;
   a sneak click with the destruction gadget stores `destroy_plan: Option<Vec<[i32;3]>>` in the
   `buildinggadgets2:gadget_state` component and answers the count on the action bar; a plain
   click writes nothing. The status command prints the count while that gadget is held, and the
   range command/wheel accept either gadget. The building gadget's replies are unchanged.
3. **Content:** `gadget_destruction` item, its recipe (`iri/ere/ili` with the jar's `c:` tags
   resolved to concrete items, as the building recipe does), advancement, model, the jar's own
   16×16 texture (sha `85a0f235…`, byte-identical to the jar), lang and the creative tab.

**Red → green (unit).** Six new tests were run first against stubs (`destroyable -> false`,
`destroy_plan -> Vec::new()`, both replies `""`), from
`ports/_shared/evidence/buildinggadgets2/sdk-test-red-0.2.7-destroy.log` (`EXIT:101`), **47 passed,
6 failed**. The headline failure:

```
---- runtime::tests::destroy_plan_counts_only_destroyable_cells stdout ----
thread 'runtime::tests::destroy_plan_counts_only_destroyable_cells' (22706) panicked at src/runtime.rs:1936:9:
assertion `left == right` failed: the tag member and the unbreakable block are excluded, two ordinary stones included
  left: []
 right: [[-2, 150, 0], [-1, 150, 0], [0, 150, 0]]
```

Green: **53 passed, 0 failed** (`sdk-test.log`). The new tests are
`destroyable_refuses_air_and_unbreakable_but_takes_stone`, `a_deny_tag_member_cannot_be_destroyed`,
`the_render_block_line_is_blocked_at_this_pin`, `destroy_plan_counts_only_destroyable_cells`,
`destroy_plan_with_ordinary_stones_includes_both` and `destroy_reply_and_status_name_the_planned_count`.

### In game (fail-capable)

Two phases in `tools/e2e.sh`, one at (1,150,0) and one at (2,150,0) beside the clicked stone at
(0,150,0); the whole y=150 layer is cleared first so a previous phase cannot leak a block into
the count. Both rigs are read back before the click and again after it, and the unknown gadget
id is refused on the control game before any recipe runs.

```
== destruction phase destroy_refusals: two cells beside the clicked stone, 1 planned ==
  cells (1,150,0) and (2,150,0): minecraft:piston_head, minecraft:bedrock
  plan reply: Destruction Gadget: planned 1 cell, 0 removed (blocked: no-cancellable-placement)
  status: Destruction Gadget: range 5, planned 1 cells, 0 removed (blocked: no-cancellable-placement)
== destruction phase destroy_control: two cells beside the clicked stone, 3 planned ==
  cells (1,150,0) and (2,150,0): minecraft:stone, minecraft:stone
  plan reply: Destruction Gadget: planned 3 cells, 0 removed (blocked: no-cancellable-placement)
  status: Destruction Gadget: range 5, planned 3 cells, 0 removed (blocked: no-cancellable-placement)
PASS: … the destruction gadget's parsed status named 1 planned cell with a deny-tag piston head and a -1-destroy-speed bedrock beside the click, 3 with two ordinary stones there, 0 removed in both …
EXIT:0
```

The pair is the proof: a read that always refuses the tag/speed reports 3 in the first rig, and
one that always accepts them reports 1 in the control rig. `SELFTEST=1` swaps both expectations
(1->3 and 3->1), so one run proves both directions reject. The carried destruction recipe is
crafted with all four ingredients consumed (and a lapis-free control matches nothing), and the
control game refuses `buildinggadgets2:gadget_destruction` as an unknown id.

### Coverage — every family in the jar (0.2.7)

Replaces the 0.2.6 table; unchanged rows keep their verdicts.

| Family | Status |
|---|---|
| the eight building modes, place-on-top, borders, per-cell gate | tested — phases as in 0.2.6, 0 written, 0 spent |
| `cleanBlockState` — denied/kept properties, `age` instances | tested — `clean_selection`, `clean_transient` and the six unit tests |
| `buildinggadgets2:deny` — selection and build destination | tested — `deny_selection`, `deny_destination_control` (8), `deny_destination` (6) |
| the building gadget's status readout | tested — names the cleaned state string |
| carried building recipe | tested — crafted, ingredients consumed, missing-lapis control empty |
| **carried destruction recipe** | tested — crafted, iron×4/redstone×2/ender pearl×2/lapis×1 consumed, missing-lapis control empty |
| **destruction destination — air** | unit tested — `destroyable_refuses_air_and_unbreakable_but_takes_stone` |
| **destruction destination — `destroy_speed < 0`** | tested — phase `destroy_refusals` (`minecraft:bedrock` excluded) |
| **destruction destination — deny tag** | tested — phase `destroy_refusals` (`minecraft:piston_head` excluded), control phase includes two stones |
| **destruction destination — `RenderBlock`** | `blocked(no-render-block-surface)` — no stamped surface; the other three lines carried |
| **destruction plan readout** | tested — parsed plan and status counts in both rigs; `SELFTEST` inverts both |
| **destruction gadget id** | tested — resolved in the packaged game, refused in the control |
| the block writes and the item payment | **held** — `blocked(no-cancellable-placement)`; every phase asserts 0 written and 0 spent |
| block removal, undo list, destruction energy and range screen | not ported — this is a plan + readout slice; `removeTickHandler` is not called |
| paste/rotate/cut/exchange gadgets, anchor, GUI, wheel screen | not ported — later slices |
| preview | `blocked(no-client-held-stack-components)` — unchanged |

### Gaps (0.2.7)

The 0.2.6 list holds with one new family and one closure: the destruction destination plan is
carried (with its RenderBlock line `blocked(no-render-block-surface)`); the actual removal, undo
list, destruction energy and range screen are not. The build stays held
(`blocked: no-cancellable-placement`), the preview stays
`blocked(no-client-held-stack-components)`, and the other gadgets are unchanged.

### Verification (0.2.7)

| Check | Result |
|---|---|
| `cargo test` | **53 passed**, 0 failed (`sdk-test.log`; red first: `sdk-test-red-0.2.7-destroy.log`, 47/6, `EXIT:101`) |
| `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` | exit 0 / exit 0 (`sdk-clippy.log`, `sdk-fmt.log`) |
| `cargo horizon-mod build` twice, `rm -f Cargo.lock` before each | **57,068 B**, `cmp` identical (`sdk-build.log`, `sdk-build2.log`) |
| `cargo horizon-mod check` | `buildinggadgets2 0.2.7 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)`, exit 0 (`sdk-check.log`) |
| `fits --build 0a875da…` | `0a875da  made for` (`sdk-fits.log`) |
| `fits --build 0b14f8a…` | `0b14f8a  fits` — a different build, so the pin run is not a tautology (`sdk-fits-other.log`) |
| `verify-sdk-package.py buildinggadgets2` | 1/1, `package matches the tree` |
| `check-recipe-types.py buildinggadgets2` | `buildinggadgets2: ok`, exit 0 |
| `launcher-game.sh build buildinggadgets2` | `Compiling buildinggadgets2 (1 of 1)` after removing the stale `native-*` tags carrying this mod (`launcher-build-0.2.7.log`) |
| e2e (modded, 18 phases) | `PASS` + `EXIT:0` (`e2e-destroy.log`) |
| e2e (control, same Nightly without the package) | `FAIL: the destruction gadget id … is unknown` + `EXIT:1` (`e2e-destroy-control.log`) |
| e2e (`SELFTEST=1`) | all 18 phases rejected the inverted expectations (34 assertions), `FAIL` + `EXIT:1`; both destruction phases swap 1↔3 (`e2e-destroy-selftest.log`) |
| `check-e2e-logs.py buildinggadgets2-e2e --strict` | **13/13** — the 10 earlier legs stay green |
| `check-skills.py --notes ports/buildinggadgets2` | `Skills:` line present |

**Package.** `buildinggadgets2.hmod` **57,068 B**, sha256
`a506d017e78f5e05781ad68870836e7c9d39dc044b00f4a5522ef3948146e81f`, **25 entries** (the six
new destruction files included: item, model, texture, data item, recipe, advancement), no packed
`Cargo.lock`; declared version **0.2.7**; `src/runtime.rs` sha256 `21437f145fd5…` in the tree and
in the archive. The game leg was rebuilt from this tree after the docs were written
(`Compiling buildinggadgets2 (1 of 1)`), so the bytes under test are the bytes delivered.

### The defect found while adding the leg (fixed, not reproduced)

The first in-game run planned nothing because `write_held` still filtered to the building gadget
(`held_gadget`), so neither the range command nor the plan write reached the destruction stack.
After the fix (`held_any_gadget`) the probe confirmed the component on the held destruction
stack reads `range:5` and the status names the plan. The range command is now issued **after**
the destruction gadget is selected — setting it while the building gadget is still held writes
that stack instead. Both were harness/code defects of this slice, proved by the red leg and the
green pair; neither is a Java-mod defect.

### Changelog (0.2.7)

> **Building Gadgets 2 0.2.7** — the destruction gadget is carried: its item, recipe and a sneak
> click destination plan. The plan refuses air, a `destroy_speed < 0` block (bedrock) and a
> `buildinggadgets2:deny` member (piston head), stores its cells in `gadget_state` and names the
> count in `/buildinggadgets2`; upstream's `RenderBlock` clause is
> `blocked(no-render-block-surface)` (no stamped query) and remains a recorded line only. The
> parsed in-game pair is 1 planned cell with the two refused blocks beside the click and 3 with
> two ordinary stones there, 0 removed in both. The build stays held
> (`blocked: no-cancellable-placement`) and the preview stays
> `blocked(no-client-held-stack-components)`.

## Slice 7 — the exchange gadget's destination plan (2026-11-24, version 0.2.8)

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

**Gap it closes.** `### Coverage — every family in the jar (0.2.7)`:
*"`paste/rotate/cut/exchange gadgets, anchor, GUI, wheel screen` | not ported — later slices"*.
The brief names the exchange gadget as the remaining consumer of `buildinggadgets2:deny`
from the same line. Class content, tier deep; the same hold as 0.2.7 lets it land as a
**plan + readout only**: a new item, a `gadget_state` field and a pure plan, with no
`BuildingUtils.exchange` call, no `removeTickHandler`, no renderer and no entity system.
The destruction gadget's planner, replies and three measured queries are reused.

### The plan rules as read from the jar and the same `1.21.1` clone (`11cd2efb…`)

- `common/items/GadgetExchanger.java:72-85`: `onAction` collects with `mode.collect(...)`
  and hands the cells to `BuildingUtils.exchange` — the write this slice withholds.
  `:91-101`: `onShiftAction` answers *Invalid Block* for a failed `isValidBlockState`.
- `util/modes/BaseMode.java:105-125` `isExchangingValid`: air is refused, then
  `GadgetUtils.isValidBlockState`, then the block-entity/fuzzy/compare clauses (settings
  and a render-block class check, out of this slice). `isPosValid` adds height,
  `canSurvive` and `mayInteract` (the held path's gate).
- `util/BuildingUtils.java:467-513` `exchange`: per destination `mayInteract`, a
  `BreakEvent`, the already-matching check and `isValidBlockState` decide; payment,
  energy and the queued write are the hold.
- `util/GadgetUtils.java:49-53` `isValidBlockState`: `!BG2DENY && getDestroySpeed >= 0 &&
  (fluid empty || source)`.

**The id comes from the artefact, not the brief's class name.** The jar's item, recipe
result and lang id are `buildinggadgets2:gadget_exchanging`; `GadgetExchanger` is the
Java *class*. The plan therefore declares that id, and the report records the
discrepancy. Its recipe is the jar's `iri / dld / ili` with `c:ingots/iron`,
`c:dusts/redstone`, `c:gems/diamond` and `c:gems/lapis` resolved to concrete items as
for the other two recipes; the jar's model, advancement and 16×16 texture are carried
byte-for-byte (texture sha256 `6cdf489e52adc95c06c8983f019f0726ad07ca1ec8e3a61356006ed77187a28f`).

| Rule | Pin query | Verdict |
|---|---|---|
| air | `BlockMetadata::name(state.block())` -> `minecraft:{air,cave_air,void_air}` | **carried** |
| `destroy_speed < 0` | `BlockStates::destroy_speed` (stamped, bedrock −1 in 26.3) | **carried** |
| deny-tag member | `Host::block_in_tag(block, "buildinggadgets2:deny")` (merged tag) | **carried** |
| `RenderBlock` (fuzzy clause) | no stamped block-class surface at `0a875da` | **`blocked(no-render-block-surface)`** — unchanged from 0.2.7, not invented |

The three queryable clauses are exactly the destruction planner's predicate, so
`layer_plan` is now shared by both planners (the verified ±range layer maths was not
re-derived). The fluid, block-entity, fuzzy/compare and `RenderBlock` clauses are
recorded, not faked.

### What shipped

1. **`runtime`:** `EXCHANGE_GADGET = "buildinggadgets2:gadget_exchanging"`, a shared
   `layer_plan`, `exchange_plan`, `exchange_reply` and `exchange_status_reply`. Both
   replies keep the hold explicit: `0 placed, 0 removed (blocked: no-cancellable-placement)`.
2. **`support`:** `GadgetState.exchange_plan` travels in the same component under the
   `exchange_plan` key; an item-scoped `ExchangeGadget` Part plans on a sneak click and
   writes nothing on a plain click; `/buildinggadgets2` names that stack's plan count;
   the range command and wheel accept the exchange gadget alongside the other two.
3. **Content:** `gadget_exchanging` item definition, the jar's model/texture, the
   concrete-id recipe, its advancement, lang name and creative-tab entry.

### Red → green (verbatim)

Six new tests were run first against stubs (`exchange_plan -> Vec::new()`, both replies
`""`) from `ports/_shared/evidence/buildinggadgets2/sdk-test-red-0.2.8-exchange.log`
(`EXIT:101`), **54 passed, 5 failed** (the id test is data and passed):

```
---- runtime::tests::exchange_plan_excludes_the_tag_member_and_the_unbreakable_block stdout ----
thread 'runtime::tests::exchange_plan_excludes_the_tag_member_and_the_unbreakable_block' (117277) panicked at src/runtime.rs:2067:9:
assertion `left == right` failed: the deny-tag piston head and the -1-speed bedrock are excluded, three stones kept
  left: []
 right: [[-2, 150, 0], [-1, 150, 0], [0, 150, 0]]
```

Green: **59 passed, 0 failed** (`sdk-test.log`). The new tests are
`the_exchange_gadget_id_is_the_jars_item_id`,
`exchange_plan_excludes_the_tag_member_and_the_unbreakable_block`,
`exchange_plan_with_ordinary_stones_includes_both`, `exchange_plan_skips_air`,
`exchange_reply_names_the_planned_count_and_the_hold` and
`exchange_status_reply_names_the_range_and_the_plan`.

### In game (fail-capable, on the delivered bytes)

Two phases in `tools/e2e.sh`, the same rig as the destruction pair: the whole y=150
layer is cleared first, a stone sits at (0,150,0) and the two cells (1,150,0)/(2,150,0)
are read back before the click. Quoted from
`ports/_shared/evidence/buildinggadgets2-e2e/e2e-exchange.log`:

```
== exchange phase exchange_refusals: two cells beside the clicked stone, 1 planned ==
  cells (1,150,0) and (2,150,0): minecraft:piston_head, minecraft:bedrock
  plan reply: Exchanging Gadget: planned 1 cell, 0 placed, 0 removed (blocked: no-cancellable-placement)
  status: Exchanging Gadget: range 5, planned 1 cells, 0 placed, 0 removed (blocked: no-cancellable-placement)
== exchange phase exchange_control: two cells beside the clicked stone, 3 planned ==
  cells (1,150,0) and (2,150,0): minecraft:stone, minecraft:stone
  plan reply: Exchanging Gadget: planned 3 cells, 0 placed, 0 removed (blocked: no-cancellable-placement)
  status: Exchanging Gadget: range 5, planned 3 cells, 0 placed, 0 removed (blocked: no-cancellable-placement)
PASS: all three carried recipes crafted one gadget each with their ingredients consumed (a missing lapis matched nothing in each); … the exchange gadget's action bar and parsed status named the same 1-with-refusals / 3-with-stones pair with 0 placed and 0 removed
EXIT:0
```

The pair is the proof: a read that always refuses the tag/speed reports 3 in the first
rig, one that always accepts them reports 1 in the control rig. `SELFTEST=1` swaps both
expectations (1→3 and 3→1) and **all 20 phases reject** (`36 assertions`, `FAIL` +
`EXIT:1`, `e2e-exchange-selftest.log`). The control game without the package refuses
`buildinggadgets2:gadget_exchanging` as an unknown id (`EXIT:1`). The carried exchange
recipe is crafted with all ingredients consumed (iron×4, redstone×1, diamond×2,
lapis×2) and a lapis-free control matches nothing. All 13 earlier e2e legs stay green.

### Coverage — every family in the jar (0.2.8)

Replaces the 0.2.7 table; unchanged rows keep their verdicts.

| Family | Status |
|---|---|
| the eight building modes, place-on-top, borders, per-cell gate | tested — phases as in 0.2.7, 0 written, 0 spent |
| `cleanBlockState` — denied/kept properties, `age` instances | tested — `clean_selection`, `clean_transient` and the six unit tests |
| `buildinggadgets2:deny` — selection and build destination | tested — `deny_selection`, `deny_destination_control` (8), `deny_destination` (6) |
| the building gadget's status readout | tested — names the cleaned state string |
| carried building recipe | tested — crafted, ingredients consumed, missing-lapis control empty |
| carried destruction recipe | tested — crafted, ingredients consumed, missing-lapis control empty |
| destruction destination — air / speed / deny tag / `RenderBlock` | tested (three queries) / `blocked(no-render-block-surface)` |
| destruction plan readout + gadget id | tested — 1↔3 pair, both replies, control refuses the id |
| **carried exchange recipe** | tested — crafted with iron×4/redstone×1/diamond×2/lapis×2 consumed, missing-lapis control empty |
| **exchange destination — air** | unit tested — `exchange_plan_skips_air` |
| **exchange destination — `destroy_speed < 0`** | tested — phase `exchange_refusals` (`minecraft:bedrock` excluded) |
| **exchange destination — deny tag** | tested — phase `exchange_refusals` (`minecraft:piston_head` excluded), control includes two stones |
| **exchange destination — `RenderBlock` / fluid / block entity / fuzzy** | `blocked(no-render-block-surface)` for the block-class line; settings-gated Java clauses not ported in this plan-only slice |
| **exchange plan readout + gadget id** | tested — parsed action bar and status counts in both rigs; `SELFTEST` inverts both; id resolved packaged, refused in control |
| the block writes and the item payment | **held** — `blocked(no-cancellable-placement)`; every phase asserts 0 written and 0 spent |
| block removal/swap, undo, energy and the mode/range screens | not ported — plan-only slices; `removeTickHandler`/`exchange` are not called |
| paste/rotate/cut gadgets, anchor, GUI, wheel screen | not ported — later slices |
| preview | `blocked(no-client-held-stack-components)` — unchanged |

### Gaps (0.2.8)

The 0.2.7 list holds with one closure: the exchange destination plan is carried (with
its `RenderBlock` line `blocked(no-render-block-surface)`); the actual swap, undo list,
energy and range screen are not. The build stays held
(`blocked: no-cancellable-placement`), the preview stays
`blocked(no-client-held-stack-components)`, and the other gadgets are unchanged.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

### Verification (0.2.8)

| Check | Result |
|---|---|
| `cargo test` | **59 passed**, 0 failed (`sdk-test.log`; red first: `sdk-test-red-0.2.8-exchange.log`, 54/5, `EXIT:101`) |
| `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` | exit 0 / exit 0 (`sdk-clippy.log`, `sdk-fmt.log`) |
| `cargo horizon-mod build` twice from the final tree, `rm -f Cargo.lock` before each | **64,190 B**, `cmp` identical (`sdk-build2.log`, `sdk-build3.log`; the earlier `sdk-build.log` predates the packed README edit) |
| `cargo horizon-mod check` | `buildinggadgets2 0.2.8 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)`, exit 0 (`sdk-check.log`) |
| `fits --build 0a875da…` / `fits --build 0b14f8a…` | `made for` / `fits` (`sdk-fits.log`, `sdk-fits-other.log`) |
| `verify-sdk-package.py buildinggadgets2` | 1/1, `package matches the tree`, R9 green |
| `check-recipe-types.py buildinggadgets2` | `buildinggadgets2: ok`, exit 0 |
| `launcher-game.sh build buildinggadgets2` | `Compiling buildinggadgets2 (1 of 1)` after removing the stale `native-*` tag and export (`launcher-build-0.2.8.log`) |
| e2e (modded, 20 phases) | `PASS` + `EXIT:0` (`e2e-exchange.log`) |
| e2e (control, same Nightly without the package) | `FAIL: the destruction gadget id … is unknown` + `EXIT:1` (`e2e-exchange-control.log`) |
| e2e (`SELFTEST=1`) | all 20 phases rejected the inverted expectations (36 assertions), `FAIL` + `EXIT:1` (`e2e-exchange-selftest.log`) |
| `check-e2e-logs.py buildinggadgets2-e2e --strict` | **16/16** — the 13 earlier legs stay green |
| `check-skills.py --notes ports/buildinggadgets2` | `Skills:` line present |

**Package.** `buildinggadgets2.hmod` **64,190 B**, sha256
`bcbc948a8a4c74628757dc96f67e8fce4118ba7ae7e448f7c13dca21ccdbd69e`, **31 entries** (the
six new exchange files included), no packed `Cargo.lock`; declared version **0.2.8**;
`src/runtime.rs` sha256 `87ddc7dcbf05…` in the tree and in the archive. The game leg
was rebuilt from this tree after the docs were written (`Compiling buildinggadgets2
(1 of 1)`), so the bytes under test are the bytes delivered.

### Changelog (0.2.8)

> **Building Gadgets 2 0.2.8** — the exchange gadget is carried under the jar's item
> id `buildinggadgets2:gadget_exchanging`: its item, recipe, the jar's model/texture and
> a sneak-click destination plan on the same one-layer rule as the destruction gadget.
> The plan refuses air, an unbreakable block (`destroy_speed < 0`) and a
> `buildinggadgets2:deny` member with the three stamped queries; upstream's
> `RenderBlock` clause is `blocked(no-render-block-surface)` and the fluid/block-entity/
> fuzzy clauses need settings this pin does not stamp. The plan is stored on the stack and
> `/buildinggadgets2` names its count with 0 placed and 0 removed. The parsed in-game pair
> is 1 planned cell with a deny-tag piston head and a bedrock beside the click and 3 with
> two ordinary stones there; nothing is swapped. The build stays held
> (`blocked: no-cancellable-placement`) and the preview stays
> `blocked(no-client-held-stack-components)`.

## Next slice

**Family:** the coarse `paste/rotate/cut gadgets` line, which is the last item of the
coverage row this slice edited and the only remaining tool family in the jar that does
not need a render or entity surface. The exact gap line is
*`paste/rotate/cut gadgets, anchor, GUI, wheel screen | not ported — later slices`*
(`### Coverage — every family in the jar (0.2.8)`). **Tier fit:** `deep`/content — the
paste gadget's `Copy`-mode destination collection is the same pure layer/plan shape as
0.2.7/0.2.8, so a plan + readout fits without the held write, a renderer or the
block-entity data copy that the real paste needs. **In-game assertion:** hold
`buildinggadgets2:gadget_paste`, sneak-click the stone at (0,150,0) with a
`minecraft:piston_head` and a `minecraft:bedrock` at (1,150,0)/(2,150,0); `ctl chat
"/buildinggadgets2"` then `ctl messages 4 | jq -r '.actionbar'` must parse to a count of
1 (both refused) and 3 with two ordinary stones there, both with 0 placed/0 removed,
and `SELFTEST=1` must invert both counts.

## Slice 8 — the Copy/Paste gadget's destination plan (2026-11-24, version 0.2.9)

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

**Gap it closes.** `### Coverage — every family in the jar (0.2.8)`:
*`paste/rotate/cut gadgets, anchor, GUI, wheel screen | not ported — later slices`*.
Class content, tier deep. The Copy/Paste gadget's Copy-mode collection is the same
pure layer/plan shape as 0.2.7/0.2.8, so a **plan + readout** lands without the held
write, the rendered paste, the anchor, the wheel screen or the block-entity data copy.
`BuildingUtils.build`/`exchange` and `removeTickHandler` are not called; the 0.2.4
placement hold stays intact.

### The plan rules as read from the jar and the same `1.21.1` clone (`11cd2efb`)

- `common/items/GadgetCopyPaste.java:69-95` `onAction`: Copy sets the start pos and calls
  `buildAndStore` (the collection); Paste reads `BG2Data`'s stored list and hands it to
  `BuildingUtils.build`/`exchange` — the write this slice withholds. `:102-116`
  `onShiftAction` sets the end pos and collects. `:118-125` `buildAndStore` calls
  `new Copy().collect(...)` and prints `messages.copyblocks` ("Copied %d blocks").
- `util/modes/Copy.java:36-73` `collectWorld`: iterate the AABB between the two clicked
  corners, keep a cell when `GadgetUtils.isValidBlockState(state)` and it is not a
  `RenderBlock`; write AIR for a refused cell so the list has an entry per position.
  The area caps (500 per axis, 100000 cells) are out of this slice's one-layer rig.
- `util/GadgetUtils.java:49-53` `isValidBlockState`: `!BG2DENY && getDestroySpeed >= 0 &&
  (fluid empty || source)`.

**The id comes from the artefact, not the brief.** The jar's item model, recipe result
and lang id are `buildinggadgets2:gadget_copy_paste` ("Copy Paste Gadget");
`GadgetCopyPaste` is the Java *class*, and the 0.2.9 brief's `gadget_paste` is shipped
by no file in the jar. The plan declares the jar's id, and the report records the
discrepancy (as 0.2.8 did for `gadget_exchanging`). Its recipe is the jar's
`iri / ere / ili` with `c:gems/emerald`, `c:ingots/iron`, `c:dusts/redstone` and
`c:gems/lapis` resolved to concrete items; the jar's model and 16×16 texture are
carried byte-for-byte (texture sha256
`1d3d6e82f9fe071befca26f2dddca1bbe0844ac25a4c59b2a98381407a63ffde`).

| Rule | Pin query | Verdict |
|---|---|---|
| air | `BlockMetadata::name(state.block())` -> `minecraft:{air,cave_air,void_air}` | **carried** |
| `destroy_speed < 0` | `BlockStates::destroy_speed` (stamped, bedrock −1 in 26.3) | **carried** |
| deny-tag member | `Host::block_in_tag(block, "buildinggadgets2:deny")` (merged tag) | **carried** |
| `RenderBlock` | no stamped block-class surface at `0a875da` | **`blocked(no-render-block-surface)`** — unchanged |

The three queryable clauses are exactly the destruction planner's predicate, so
`copy_plan` reuses `layer_plan` and `destroyable` verbatim — the verified layer maths
was not re-derived. The fluid, block-entity, fuzzy/compare and real paste clauses are
recorded, not faked.

### What shipped

1. **`runtime`:** `COPY_GADGET = "buildinggadgets2:gadget_copy_paste"`, `copy_plan`,
   `copy_reply` and `copy_status_reply`. Both replies keep the hold explicit:
   `0 placed, 0 removed (blocked: no-cancellable-placement)`.
2. **`support`:** `GadgetState.copy_plan` travels in the same component under the
   `copy_plan` key; an item-scoped `CopyGadget` Part collects on a sneak click and
   writes nothing on a plain click; `/buildinggadgets2` names that stack's collected
   count; the range command and wheel accept the fourth gadget; all earlier replies
   are unchanged.
3. **Content:** `gadget_copy_paste` item definition, the jar's model/texture, the
   concrete-id recipe, its advancement, lang name and creative-tab entry; a new
   `tools/craft_copy.py` real-table craft leg with a missing-lapis control.

### Red → green (verbatim)

Six new tests were run first against stubs (`copy_plan -> Vec::new()`, both replies
`""`) from `ports/_shared/evidence/buildinggadgets2/sdk-test-red-0.2.9-copy.log`
(`EXIT:101`), **60 passed, 5 failed** (the id test is data and passed):

```
---- runtime::tests::copy_plan_excludes_the_tag_member_and_the_unbreakable_block stdout ----
thread 'runtime::tests::copy_plan_excludes_the_tag_member_and_the_unbreakable_block' (198610) panicked at src/runtime.rs:2213:9:
assertion `left == right` failed: the deny-tag piston head and the -1-speed bedrock are excluded, three stones kept
  left: []
 right: [[-2, 150, 0], [-1, 150, 0], [0, 150, 0]]
```

Green: **65 passed, 0 failed** (`sdk-test.log`). The new tests are
`the_copy_gadget_id_is_the_jars_item_id`,
`copy_plan_excludes_the_tag_member_and_the_unbreakable_block`,
`copy_plan_with_ordinary_stones_includes_both`, `copy_plan_skips_air`,
`copy_reply_names_the_count_and_the_hold` and
`copy_status_reply_names_the_range_and_the_plan`.

### In game (fail-capable, on the delivered bytes)

Two phases in `tools/e2e.sh`, the same rig as the destruction/exchange pairs; the
whole y=150 layer is cleared first, a stone sits at (0,150,0) and the two cells
(1,150,0)/(2,150,0) are read back before the click. The parsed status is read with
`ctl chat "/buildinggadgets2"` then `ctl messages 4 | jq -r '.actionbar'`.

```
== copy phase copy_refusals: two cells beside the clicked stone, 1 collected ==
  cells (1,150,0) and (2,150,0): minecraft:piston_head, minecraft:bedrock
  plan reply: Copy Paste Gadget: copied 1 cell, 0 placed, 0 removed (blocked: no-cancellable-placement)
  status: Copy Paste Gadget: range 5, copied 1 cells, 0 placed, 0 removed (blocked: no-cancellable-placement)
== copy phase copy_control: two cells beside the clicked stone, 3 collected ==
  cells (1,150,0) and (2,150,0): minecraft:stone, minecraft:stone
  plan reply: Copy Paste Gadget: copied 3 cells, 0 placed, 0 removed (blocked: no-cancellable-placement)
  status: Copy Paste Gadget: range 5, copied 3 cells, 0 placed, 0 removed (blocked: no-cancellable-placement)
```

The pair is the proof: a read that always refuses the tag/speed reports 3 in the
first rig, one that always accepts them reports 1 in the control rig. `SELFTEST=1`
swaps both expectations (1→3 and 3→1), so one run proves both directions reject.
The carried Copy/Paste recipe is crafted with all ingredients consumed
(iron×4, redstone×2, emerald×2, lapis×1) and a lapis-free control matches nothing.
The control game refuses `buildinggadgets2:gadget_copy_paste` as an unknown id.

### Coverage — every family in the jar (0.2.9)

Replaces the 0.2.8 table; unchanged rows keep their verdicts.

| Family | Status |
|---|---|
| the eight building modes, place-on-top, borders, per-cell gate | tested — phases as in 0.2.8, 0 written, 0 spent |
| `cleanBlockState` — denied/kept properties, `age` instances | tested — `clean_selection`, `clean_transient` and the six unit tests |
| `buildinggadgets2:deny` — selection and build destination | tested — `deny_selection`, `deny_destination_control` (8), `deny_destination` (6) |
| the building gadget's status readout | tested — names the cleaned state string |
| carried building recipe | tested — crafted, ingredients consumed, missing-lapis control empty |
| carried destruction recipe | tested — crafted, ingredients consumed, missing-lapis control empty |
| carried exchange recipe | tested — crafted, ingredients consumed, missing-lapis control empty |
| destruction destination — air / speed / deny tag / `RenderBlock` | tested (three queries) / `blocked(no-render-block-surface)` |
| destruction plan readout + gadget id | tested — 1↔3 pair, both replies, control refuses the id |
| exchange destination — air / speed / deny tag / `RenderBlock` | tested (three queries) / `blocked(no-render-block-surface)` |
| exchange plan readout + gadget id | tested — 1↔3 pair, both replies, control refuses the id |
| **carried Copy/Paste recipe** | tested — crafted with iron×4/redstone×2/emerald×2/lapis×1 consumed, missing-lapis control empty |
| **Copy destination — air** | unit tested — `copy_plan_skips_air` |
| **Copy destination — `destroy_speed < 0`** | tested — phase `copy_refusals` (`minecraft:bedrock` excluded) |
| **Copy destination — deny tag** | tested — phase `copy_refusals` (`minecraft:piston_head` excluded), control includes two stones |
| **Copy destination — `RenderBlock` / fluid / block entity / fuzzy** | `blocked(no-render-block-surface)` for the block-class line; settings-gated Java clauses not ported in this plan-only slice |
| **Copy collection readout + gadget id** | tested — parsed action bar and status counts in both rigs; `SELFTEST` inverts both; id resolved packaged, refused in control |
| the block writes and the item payment | **held** — `blocked(no-cancellable-placement)`; every phase asserts 0 written and 0 spent |
| block removal/swap/paste, undo, energy and the mode/range screens | not ported — plan-only slices; `removeTickHandler`/`build`/`exchange` are not called |
| rotate/cut gadgets, anchor, GUI, wheel screen | not ported — later slices |
| preview | `blocked(no-client-held-stack-components)` — unchanged |

### Gaps (0.2.9)

The 0.2.8 list holds with one closure: the Copy/Paste destination collection is
carried (with its `RenderBlock` line `blocked(no-render-block-surface)`); the real
paste, its block-entity data copy, the undo list, energy and the range screen are
not. The build stays held (`blocked: no-cancellable-placement`), the preview stays
`blocked(no-client-held-stack-components)`, and the other gadgets are unchanged.

### Verification (0.2.9)

| Check | Result |
|---|---|
| `cargo test` | **65 passed**, 0 failed (`sdk-test.log`; red first: `sdk-test-red-0.2.9-copy.log`, 60/5, `EXIT:101`) |
| `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` | exit 0 / exit 0 (`sdk-clippy.log`, `sdk-fmt.log`) |
| `cargo horizon-mod build` twice from the final tree, `rm -f Cargo.lock` before each | **70,665 B**, `cmp` identical (`sdk-build.log`, `sdk-build2.log`, `sdk-build-final.log`) |
| `cargo horizon-mod check` | `buildinggadgets2 0.2.9 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)`, exit 0 (`sdk-check.log`) |
| `fits --build 0a875da…` / `fits --build 0b14f8a…` | `made for` / `fits` (`sdk-fits.log`, `sdk-fits-other.log`) |
| `verify-sdk-package.py buildinggadgets2` | 1/1, `package matches the tree`, R9 green |
| `check-recipe-types.py buildinggadgets2` | `buildinggadgets2: ok`, exit 0 |
| `launcher-game.sh build buildinggadgets2` | `Compiling buildinggadgets2 (1 of 1)` after removing the stale `native-*` tag carrying this mod (`launcher-build-0.2.9.log`); new tag `native-0a875da-9a9939180a36` |
| e2e (modded, 22 phases, four recipes) | `PASS` + `EXIT:0` (`e2e-copy.log`) |
| e2e (control, same Nightly without the package) | `FAIL: the destruction gadget id … is unknown` + `EXIT:1` (`e2e-copy-control.log`) |
| e2e (`SELFTEST=1`) | all 22 phases rejected the inverted expectations (38 assertions), `FAIL` + `EXIT:1`; both copy phases swap 1↔3 (`e2e-copy-selftest.log`) |
| `check-e2e-logs.py buildinggadgets2-e2e --strict` | **19/19** — the 16 earlier legs stay green |
| `check-skills.py --notes ports/buildinggadgets2` | `Skills:` line present |

**Package.** `buildinggadgets2.hmod` **70,665 B**, sha256
`3aea36e6258ad79569b1b0846ab05e03c201286de8ac03ee03555cf9a02d29f8`, **37 entries** (the
six new Copy/Paste files included: item, model, texture, data item, recipe, advancement),
no packed `Cargo.lock`; declared version **0.2.9**; `src/runtime.rs` sha256
`63897d041271…` in the tree and in the archive. The game leg was rebuilt from this tree
after the docs were written (`Compiling buildinggadgets2 (1 of 1)`), so the bytes under
test are the bytes delivered.

### Changelog (0.2.9)

> **Building Gadgets 2 0.2.9** — the Copy/Paste gadget is carried under the jar's
> item id `buildinggadgets2:gadget_copy_paste` (`GadgetCopyPaste` is the Java class;
> the brief's `gadget_paste` is shipped by no file): its item, recipe, the jar's
> model/texture and a sneak-click Copy-mode destination collection on the same
> one-layer rule as the destruction and exchange gadgets. The collection refuses
> air, an unbreakable block (`destroy_speed < 0`) and a `buildinggadgets2:deny`
> member with the three stamped queries; upstream's `RenderBlock` clause is
> `blocked(no-render-block-surface)` and the fluid/block-entity/fuzzy clauses need
> settings this pin does not stamp. The collected cells are stored on the stack and
> `/buildinggadgets2` names their count with 0 placed and 0 removed. The parsed
> in-game pair is 1 collected cell with a deny-tag piston head and a bedrock beside
> the click and 3 with two ordinary stones there; nothing is pasted. The build, the
> swap and the real paste stay held (`blocked: no-cancellable-placement`) and the
> preview stays `blocked(no-client-held-stack-components)`.
