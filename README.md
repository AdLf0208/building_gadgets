# Building Gadgets — a Horizon port

**Building Gadgets** is a Minecraft mod. A recreation of my very first mod - Building Gadgets. This time with less DireCode. Or more, depending on how you look at it. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Direwolf20-MC/BuildingGadgets2](https://github.com/Direwolf20-MC/BuildingGadgets2) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Building Gadgets ([source](https://github.com/Direwolf20-MC/BuildingGadgets2)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `buildinggadgets2.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- `buildinggadgets2:gadget_building` (stack size 1) right-clicked on a block:
- **sneak + right click** selects the clicked block's state (upstream's
- **plain right click** builds the selected block's **horizontal row**:
- each placed cell consumes one item of the block's kind; creative pays
- range defaults to **1** (upstream) and is set with
- `/buildinggadgets2` and `/buildinggadgets2 range` report the range and the
- **The host call that replaced `Game::native_set_block` is
- The bulk alternative in the same area is

## Not yet ported

- **Mode fixed to `horizontal_row`.** Upstream's default is `build_to_me`; the
- **Selection and range are per-player in memory**, not item components: the
- **Selection stores the raw state id**, so transient properties (age, lit,
- **Placement is immediate** through `Game::native_set_block`: no render
- **No energy.** The pin has no FE hook; upstream also charges 50 FE/block.
- No anchor, undo list, bound inventory/AE2/Curios sourcing, `place_on_top`
- Item payment uses `native_count_items`/`native_take_items` (plain stacks
- **`canSurvive` is no longer pre-checked.** 0.1.0 asked the game's own

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `buildinggadgets2.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Building Gadgets was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Direwolf20-MC/BuildingGadgets2

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run buildinggadgets2` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/buildinggadgets2/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
