# Building Gadgets 2 for Horizon

A native Horizon port of **Building Gadgets 2** (`Direwolf20-MC/BuildingGadgets2`,
MIT). Slice 1 implements the **building gadget's horizontal row**: sneak right
click a block to select it, then right click to build a row of that block away
from the player, paying one item per placed block.

This is a clean-room Rust rewrite against the pinned Horizon APIs. The Java
repository was read for behaviour only; no code was translated and no jar was
decompiled. Local-only until the maintainer approves publishing.

## The package

One flat `.hmod` (the SDK format): `horizon-mod.toml` is the manifest, `src/` is the
code, and `assets/` + `data/` are the gadget it carries. The launcher builds it
against the pinned game build; the SDK in `.horizon/sdk/` is fetched by
`cargo horizon-mod` and never packed.

```text
horizon-mod.toml               id, version 0.2.0, api 0.2, side both, [native] pins
Cargo.toml                     one crate; the only dependency is horizon-mod-api
src/lib.rs                     the one `impl Hooks` (register, component_types, commands, wheel)
src/support.rs                 game glue: component, inventory, placement, commands, wheel
src/runtime.rs                 pure row planner + command parser (byte-identical, 11 tests)
assets/buildinggadgets2/       item definition, model, texture, lang
data/buildinggadgets2/         item behaviour row, recipe, advancement, creative tab
LICENSE, LICENSE-NOTICE.md     the MIT text and where the port came from (packed)
icon.png                       optional package icon (packed)
buildinggadgets2.hmod          the delivered package
tools/e2e.sh                   fail-capable in-game check (5-block row, 5 items)
notes.md, FINDINGS.md, VALIDATION.md, REPIN-AUDIT.md
```

## Checks

Run these in the port; the SDK layout needs no staging (`CARGO_INCREMENTAL=0`,
no `--target-dir`). `build-slot.sh` speaks the retired `.hnmod` tool and does not
apply here.

```sh
CARGO_INCREMENTAL=0 cargo test                            # 11 runtime tests
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
cargo fmt --check                                          # NOT --all: it would format the SDK
rm -f Cargo.lock                                           # a packed lock changes the artifact
CARGO_INCREMENTAL=0 cargo horizon-mod build                # -> target/horizon-mod/buildinggadgets2.hmod
cp target/horizon-mod/buildinggadgets2.hmod ./buildinggadgets2.hmod
cargo horizon-mod check buildinggadgets2.hmod              # the registry's own check
cargo horizon-mod fits buildinggadgets2.hmod --build nightly
```

## In-game check

`tools/e2e.sh` needs a **launcher-built** game that carries the package
(`horizon-launcher native-build <port> --instance <i> --trust-native-code`), because
a native mod is compiled into `horizon-client`:

```sh
GAME=<instance>/horizon-client tools/e2e.sh             # 5 blocks placed, 5 consumed
GAME=… COUNTERFACTUAL=1 tools/e2e.sh                    # no stone: 0 placed, 0 consumed
GAME=… SELFTEST=1 tools/e2e.sh                          # must FAIL (6 expected)
```

It launches a headless survival world, sets `/buildinggadgets2 range 6` from player
chat, sneak-clicks a stone block to select it, then clicks its south face: exactly
five stone appear in the row and exactly five stone items leave the inventory.

## Scope and gaps

Slice 1 is the horizontal row only. Upstream's default `build_to_me`, the other
modes, mode switching, the anchor, undo, energy, the GUI/tooltip, bound
inventories and the build animation are deferred; see `notes.md` (its
`## Migration to the SDK format` section) for the exact gaps.
