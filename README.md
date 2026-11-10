# buildinggadgets2 (Horizon port)

A Horizon port of [buildinggadgets2](Direwolf20-MC/BuildingGadgets2) — MIT.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- `buildinggadgets2:gadget_building` (stack size 1) right-clicked on a block:
- sneak + right click** selects the clicked block's state (upstream's
- `onShiftAction`); air, unbreakable and non-source fluids are refused;
- plain right click** builds the selected block's **horizontal row**:
- `side = face.axis == Y ? player facing : -face`, cells
- `clicked + side*i` for `i in 0..range`, skipping cells that are not
- replaceable, outside the build height, or where the state cannot survive;
- each placed cell consumes one item of the block's kind; creative pays

## Not yet ported

- Mode fixed to `horizontal_row`.** Upstream's default is `build_to_me`; the
- other modes (vertical wall/column, surface, horizontal wall, stairs, grid,
- build-to-me) and mode switching are deferred. The mode wheel is client-only.
- Selection and range are per-player in memory**, not item components: the
- pin's `use_item_on` gives `&ItemStack` and has no component-write path, and
- there is no scroll/key hook. They reset on relog.
- Selection stores the raw state id**, so transient properties (age, lit,
- waterlogged, open, ...) are not cleaned as upstream's `cleanBlockState`

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: Direwolf20-MC/BuildingGadgets2. Port: AdLf0208. Licence: MIT — see [LICENSE](LICENSE);
the original notices are preserved.
