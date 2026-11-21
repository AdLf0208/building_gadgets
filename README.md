# Building Gadgets 2 for Horizon

A native Horizon port of **Building Gadgets 2** (`Direwolf20-MC/BuildingGadgets2`,
MIT). The building gadget is complete: sneak right click a block to select it, then
right click to build in one of upstream's eight modes (`build_to_me` by default,
`vertical_wall`, `vertical_column`, `surface`, `horizontal_wall`, `stairs`,
`horizontal_row`, `grid`), paying one item per placed block.

This is a clean-room Rust rewrite against the pinned Horizon APIs. The Java
repository was read for behaviour only; no code was translated and no jar was
decompiled. Local-only until the maintainer approves publishing.

## The package

One flat `.hmod` (the SDK format): `horizon-mod.toml` is the manifest, `src/` is the
code, and `assets/` + `data/` are the gadget it carries. The launcher builds it
against the pinned game build; the SDK in `.horizon/sdk/` is fetched by
`cargo horizon-mod` and never packed.

```text
horizon-mod.toml               id, version 0.2.3, api 0.2, side both, [native] pins
Cargo.toml                     one crate; the only dependency is horizon-mod-api
src/lib.rs                     the one `impl Hooks` (register, component_types, commands, wheel)
src/support.rs                 game glue: component, inventory, placement, commands, wheel
src/runtime.rs                 pure mode planners + command parser (30 tests)
assets/buildinggadgets2/       item definition, model, texture, lang
data/buildinggadgets2/         item behaviour row, recipe, advancement, creative tab
LICENSE, LICENSE-NOTICE.md     the MIT text and where the port came from (packed)
icon.png                       optional package icon (packed)
buildinggadgets2.hmod          the delivered package
tools/e2e.sh                   fail-capable in-game check (recipe + one phase per mode)
tools/craft_gadget.py          crafts the carried recipe in a real crafting table
notes.md, FINDINGS.md, VALIDATION.md, REPIN-AUDIT.md
```

## Checks

Run these in the port; the SDK layout needs no staging (`CARGO_INCREMENTAL=0`,
no `--target-dir`).

```sh
CARGO_INCREMENTAL=0 cargo test                            # 30 runtime tests
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
cargo fmt --check                                          # NOT --all: it would format the SDK
rm -f Cargo.lock                                           # a packed lock changes the artifact
CARGO_INCREMENTAL=0 cargo horizon-mod build                # -> target/horizon-mod/buildinggadgets2.hmod
cp target/horizon-mod/buildinggadgets2.hmod ./buildinggadgets2.hmod
cargo horizon-mod check buildinggadgets2.hmod              # the registry's own check
cargo horizon-mod fits buildinggadgets2.hmod --build 0a875da
```

## In-game check

`tools/e2e.sh` needs a **launcher-built** game that carries the package, because a
native mod is compiled into `horizon-client`:

```sh
ports/tools/launcher-game.sh build buildinggadgets2
eval "$(ports/tools/launcher-game.sh env buildinggadgets2)"
bash tools/e2e.sh                    # recipe + 9 mode phases: PASS
GAME=$CONTROL_GAME bash tools/e2e.sh # no package: FAIL (recipe/id unknown)
SELFTEST=1 bash tools/e2e.sh         # every phase expects one cell too many: FAIL
```

It launches an `--offscreen` survival world (a crafting screen needs a renderer),
crafts the carried recipe in a real crafting table (and shows the same pattern
without lapis matches nothing), then runs one phase per mode at a floating stone
block: each phase asserts the exact set of cells the click placed and one stone
consumed per cell, plus the action-bar reply of `mode`/`range`/`placeontop`.

## Scope and gaps

All eight building modes and the mode/place-on-top commands are ported. The radial
mode wheel (a client screen), the anchor, undo, energy, the GUI/tooltip, bound
inventories, the preview/build-animation queue and the paste/rotate/cut/
destruction/exchange gadgets are deferred; see `notes.md` (its `## Slice 2 — the
mode family` and `### Gaps` sections) for the exact list.
