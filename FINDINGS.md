# Building Gadgets 2 — upstream analysis (slice 1: horizontal row)

Analysed 2026-11-05. The clone lives under
`ports/_shared/upstream/buildinggadgets2/` (git-ignored) and was deleted after
this note was written.

## Repository, branch, licence

- Repository: <https://github.com/Direwolf20-MC/BuildingGadgets2>
- Licence: **MIT** (`LICENSE`, Copyright (c) 2023 Direwolf20-MC). The port stays
  local-only until the maintainer approves publishing (ARCHITECTURE.md §8).
- Branch analysed: **`1.21.1`**, commit
  **`11cd2efb2719e480018a81cfe5a05bd087cb1388`** (2026-10-24, tag `v21.1.1`),
  the ATM10 line (`horizon/docs/modpacks/atm10-mods.csv` ships **1.3.9**; the
  branch is that same 1.21.1 line at a newer commit).
- `main` is the 26.1.2 line (`529c2962ffe0f5d2ef20125ccab9ad2426bc7a56`, tag
  `v26.1.2.3`). `HorizontalRow.java` is byte-for-byte the same rule there; it
  was read to confirm the placement rule did not change between the two lines.
- No jar was decompiled. Only the public repository source and resources were
  read.

## Files read

`setup/Registration.java`, `setup/ModSetup.java`, `setup/Config.java`,
`setup/BG2DataComponents.java`, `common/items/BaseGadget.java`,
`common/items/GadgetBuilding.java`, `util/modes/BaseMode.java`,
`util/modes/HorizontalRow.java`, `util/modes/BuildToMe.java`,
`util/modes/VerticalColumn.java`, `util/GadgetNBT.java`,
`util/GadgetUtils.java`, `util/BuildingUtils.java`,
`common/events/ServerTickHandler.java`, `common/events/ServerBuildList.java`,
`common/network/handler/PacketRangeChange.java`,
`client/screen/ModeRadialMenu.java`, `client/events/EventKeyInput.java`,
`datagen/BG2Recipes.java`, `datagen/BG2LanguageProvider.java`,
`assets/buildinggadgets2/models/item/gadget_building.json`, the item texture,
and the Patchouli entry `mode_horz_row.json`.

## Behaviour extracted

### Item and defaults

- One item, `buildinggadgets2:gadget_building` (`GadgetBuilding`), stack size 1.
- `GadgetNBT.getGadgetBlockState` defaults to **air**: a fresh gadget places
  nothing until a block is selected.
- `GadgetNBT.getToolRange` defaults to **1**; the radial menu's slider allows
  **1..=15** and `RangeChangePayload` writes it.
- Energy: `maxPower` 500000, `baseCost` 50 per placed block
  (`Config.powerConfig`); creative pays no energy and no items.
- Default mode of a building gadget with no mode component is **`build_to_me`**
  (`GadgetNBT.getMode`); the registered building modes are BuildToMe,
  VerticalWall, VerticalColumn, Surface, HorizontalWall, Stairs, HorizontalRow
  and Grid (`GadgetModes`).

### Selection (sneak right click)

`BaseGadget.use` → shift → `GadgetBuilding.onShiftAction`:

1. read the clicked block state;
2. reject it when `GadgetUtils.isValidBlockState` fails: it is in the
   `buildinggadgets2:deny` block tag, its destroy speed is < 0, or it is a
   non-source fluid;
3. `cleanBlockState`: the block's default state, with every property copied
   except the DENY list (the age properties, `half`, `waterlogged`, `lit`,
   `has_record`, `has_book`, `open`, `stage`);
4. store it in the gadget's `gadget_blockstate` component.

### Horizontal row (`HorizontalRow.collectWorld`)

| Input | Rule |
|---|---|
| `range` | `GadgetNBT.getToolRange` (default 1, max 15) |
| `placeontop` | toggle, default false: start one block above the clicked block |
| `side` | `hitSide.axis == Y ? player.getDirection() : hitSide.getOpposite()` |
| cells | `for i in 0..range`: `startAt.relative(side, i)`, keeping `isPosValid` |
| exchanging | `side.getClockWise()`, `i in -range/2..=range/2`, optional connected-area BFS |

`isPosValid` (building mode): the block at the cell `canBeReplaced()`, the cell
is inside the build height, the selected state `canSurvive(level, pos)`, and
`level.mayInteract(player, pos)`. Invalid cells are skipped and the scan
continues.

The Patchouli entry confirms the intent: "build blocks away from the player
based on the direction they are looking … useful for building bridges."

### Build and payment (`BuildingUtils.build` + `ServerTickHandler.build`)

- Each collected `StatePos` is relative to the clicked block; the server adds
  `lookingAt` back when it places.
- Skip air; skip when the target is no longer replaceable or the state cannot
  survive (one retry pass); `mayInteract` and the NeoForge place event are
  checked.
- **Needed item** = `GadgetUtils.getDropsForBlockState`: the block's own drops
  with a silk-touch gadget, falling back to the block's clone item. For stone
  that is one `minecraft:stone`.
- Survival: a simulated `removeStacksFromInventory` must succeed, else the cell
  is skipped. The real removal happens when the cell is placed (main inventory
  first, then a bound inventory/AE2, Curios, offhand).
- Energy: if the gadget has less than `baseCost`, the build stops; creative is
  free.
- Placement is deferred through `ServerTickHandler` at 1..50 cells per tick:
  a `RenderBlock` is placed first and the real state is written after the
  animation, with an undo list. Creative pays nothing.

## How the rewrite differs (slice 1)

- **Mode fixed to `horizontal_row`.** Upstream's default is `build_to_me`;
  switching modes, the radial menu and the other modes are later slices.
- **Selection and range are per-player in memory**, not item components: the
  pin's `use_item_on` gives `&ItemStack` and no component write path. The
  scroll-wheel range is replaced by `/buildinggadgets2 range <1..15>` because
  the pin has no mod key or scroll hook.
- **The raw clicked state id is stored**, not a `cleanBlockState` copy: the pin
  exports states but not a per-property copy helper, so transient properties
  (age/lit/waterlogged) are not stripped (recorded gap).
- **Placement is immediate** with `Game::native_set_block`; there is no render
  block, animation or per-tick queue, and no retry pass for torches placed
  before their support. `native_set_block` does not fire `BlockItem.place` or
  `setPlacedBy`, and block-entity data is not carried.
- **No energy**: the pin has no FE hook, so the port pays items only (upstream
  also charges 50 FE per block). Deferred.
- No anchor, undo, bound inventory/AE2/Curios sourcing, `place_on_top` toggle,
  connected-area, GUI/tooltip, sounds or particles. `mayInteract`/chunk
  protection has no pin API.
- Item payment uses `native_count_items`/`native_take_items` (plain stacks
  only), one item per placed cell.
