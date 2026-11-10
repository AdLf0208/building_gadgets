# Building Gadgets 2 — slice 1 content

Adapted from **Building Gadgets 2** (`Direwolf20-MC/BuildingGadgets2`, MIT),
branch `1.21.1` at commit `11cd2efb2719e480018a81cfe5a05bd087cb1388`.

- `data/buildinggadgets2/horizon/item/gadget_building.json` — the item on the
  vanilla `minecraft:stick` template, stack size 1.
- `assets/buildinggadgets2/items/gadget_building.json` — 26.3 item model
  definition.
- `assets/buildinggadgets2/models/item/gadget_building.json` and
  `textures/item/gadget_building.png` — the upstream model (parent
  `minecraft:item/generated`) and 16×16 texture.
- `data/buildinggadgets2/recipe/gadget_building.json` — the upstream shaped
  recipe (`iri / drd / ili` with iron, redstone, diamond, lapis); the upstream
  `#c:` tags are written as the concrete vanilla items 26.3 ships.
- `data/buildinggadgets2/advancement/recipes/misc/gadget_building.json` — the
  recipe-unlock advancement in the 26.3 format.
- `data/buildinggadgets2/horizon/creative_tab/tab.json` — the creative tab
  entry.
- `assets/buildinggadgets2/lang/en_us.json` — the item, tab, mode and tooltip
  names used by the slice.

Local-only until the maintainer approves publishing. Build the deterministic
archive with `python3 tools/package_content.py`.
