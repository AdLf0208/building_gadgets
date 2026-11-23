# Building Gadgets 2 for Horizon

A native Horizon port of **Building Gadgets 2** (`Direwolf20-MC/BuildingGadgets2`,
MIT). The building gadget is ported: sneak right click a block to select it, then
`/buildinggadgets2 mode` picks one of upstream's eight modes (`build_to_me` by
default, `vertical_wall`, `vertical_column`, `surface`, `horizontal_wall`,
`stairs`, `horizontal_row`, `grid`) within the 1..15 range. The carried
`buildinggadgets2:deny` block tag refuses a sneak click on any of its members
(upstream's `Invalid Block`) and a build destination whose block it names.

**The build is held in this version.** The SDK at the campaign pin has no
placement call that names the acting player *and* lets a protection mod cancel
one destination, so a plain right click writes no block and spends no item; it
answers on the action bar with `blocked: no-cancellable-placement` and the number
of cells the player's own placement gate would have allowed. `notes.md` →
`## Review repair (0.2.4)` records the symbols that were checked, why each one
falls short, and the issue shape that would lift the hold.

This is a clean-room Rust rewrite against the pinned Horizon APIs. The Java
repository was read for behaviour only; no code was translated and no jar was
decompiled. Local-only until the maintainer approves publishing.

## The package

One flat `.hmod` (the SDK format): `horizon-mod.toml` is the manifest, `src/` is the
code, and `assets/` + `data/` are the gadget it carries. The launcher builds it
against the pinned game build; the SDK in `.horizon/sdk/` is fetched by
`cargo horizon-mod` and never packed.

```text
horizon-mod.toml               id, version 0.2.5, api 0.2, side both, [native] pins
Cargo.toml                     one crate; the only dependency is horizon-mod-api
src/lib.rs                     the one `impl Hooks` (register, component_types, commands, wheel)
src/support.rs                 game glue: component, inventory, the held build, commands, wheel
src/runtime.rs                 pure mode planners, the placement hold, the border gate and the deny tag (41 tests)
assets/buildinggadgets2/       item definition, model, texture, lang
data/buildinggadgets2/         item behaviour row, recipe, advancement, tab, the deny tag
LICENSE, LICENSE-NOTICE.md     the MIT text and where the port came from (packed)
icon.png                       optional package icon (packed)
buildinggadgets2.hmod          the delivered package
tools/e2e.sh                   fail-capable in-game check (recipe, every mode, borders, the deny tag)
tools/craft_gadget.py          crafts the carried recipe in a real crafting table
notes.md, FINDINGS.md, VALIDATION.md, REPIN-AUDIT.md
```

## Checks

Run these in the port; the SDK layout needs no staging (`CARGO_INCREMENTAL=0`,
no `--target-dir`).

```sh
CARGO_INCREMENTAL=0 cargo test                            # 41 runtime tests
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
cargo fmt --check                                          # NOT --all: it would format the SDK
rm -f Cargo.lock                                           # a packed lock changes the artifact
CARGO_INCREMENTAL=0 cargo horizon-mod build                # -> target/horizon-mod/buildinggadgets2.hmod
cp target/horizon-mod/buildinggadgets2.hmod ./buildinggadgets2.hmod
cargo horizon-mod check buildinggadgets2.hmod              # the registry's own check
cargo horizon-mod fits buildinggadgets2.hmod --build 0a875da
cargo horizon-mod fits buildinggadgets2.hmod --build 0b14f8a
```

## In-game check

`tools/e2e.sh` needs a **launcher-built** game that carries the package, because a
native mod is compiled into `horizon-client`:

```sh
ports/tools/launcher-game.sh build buildinggadgets2
eval "$(ports/tools/launcher-game.sh env buildinggadgets2)"
bash tools/e2e.sh                    # recipe + 9 modes + 2 borders + 3 deny-tag phases: PASS
GAME=$CONTROL_GAME bash tools/e2e.sh # no package: FAIL (recipe/id unknown)
SELFTEST=1 bash tools/e2e.sh         # every phase inverted: FAIL
```

It launches an `--offscreen` survival world (a crafting screen needs a renderer),
crafts the carried recipe in a real crafting table (and shows the same pattern
without lapis matches nothing), then runs one phase per mode at a floating stone
block and two world-border phases. Every phase asserts that the click **wrote no
block** and **spent no stone**, and that the action bar carried the exact cell
count the gate would have allowed (the upstream shape's cell count for the modes,
and the two sides of the border's upper edge for the border phases). The
`mode`/`range`/`placeontop`/status replies are asserted by value too. Three
deny-tag phases close the tag family: a sneak click on `minecraft:piston_head`
answers `Invalid Block` and leaves `minecraft:stone` selected, and the grid's
held count is 8 with ordinary `minecraft:snow` at two destinations but 6 once a
test-world data pack names snow in the tag (the pair that proves the read).

## Scope and gaps

All eight building modes, the mode/place-on-top commands, the
`buildinggadgets2:deny` tag (selection and destination) and the per-destination
placement gate (build permission, `mayUseItemAt`, the world border with its
exclusive upper edges, a dedicated server's spawn protection) are ported; the
write itself is held as described above. The radial mode wheel (a client screen),
the anchor, undo, energy, the GUI/tooltip, bound inventories, the preview/
build-animation queue and the paste/rotate/cut/destruction/exchange gadgets are
deferred; see `notes.md` (its `## Review repair (0.2.4)` and `### Gaps` sections)
for the exact list.
