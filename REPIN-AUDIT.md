# Building Gadgets 2 — re-pin audit against Horizon `83ad384f`

Read-only audit, 2026-11-05. **No port source, content, manifest or tool was
changed; no build, test or e2e was run.**

| | Old | New |
|---|---|---|
| Horizon | `c9d474dd9116e84c243f8992d75c11314df56f06` (108 hooks) | **`83ad384f48d0742746d518737511882680aab5b7`** (427 hooks, 159 helpers, `NATIVE_HOOKS_API = 409`) |
| horizon-mixins | `d9c83d7096b7ea5b83678da2efdfd6161f74997e` | **`e8cd0aa38cffef49cb2e088ef716baa107148c02`** |
| plan schema | 6 | 6 (unchanged) |

Sources: `ports/_shared/pins/horizon-83ad384f/native-hooks.json`,
`crates/horizon-client/src/**`, `crates/horizon-world/src/**`,
`docs/native-mods.md`, `docs/mods.md`, `crates/horizon-mods/src/content.rs`;
this port's `notes.md` and `FINDINGS.md`.

Classification: **usable** = a writable/cancellable hook or a helper exists;
**hook-only** = only a listen hook; **absent** = no hook/helper.

Method: each deferral in `notes.md`/`FINDINGS.md` was mapped to
`native-hooks.json` and the `horizon-client`/`horizon-world` helpers;
content-only items were checked in `horizon-mods`/`docs/mods.md`. The brief
states main `696d0216` has the same hook set; this audit uses the pinned
export.

## Re-pin safety (existing injections)

| Existing hook | New-pin signature | Verdict |
|---|---|---|
| `use_item_on` | `native_hooks.rs:6482` | unchanged |
| `register_commands` | `native_hooks.rs:3899` | unchanged |
| `perform_command` | `native_hooks.rs:3909` | unchanged |

The three injection bindings still resolve. The wrapper must be migrated:
`tools/horizon.py:122-123` still runs `-p horizon-client --bin
horizon-client`, which the re-pin recipe says now runs **0 tests**; use
`--lib`. `horizon-ctl` is now `-p horizon-client-lib --bin horizon-ctl`.
`compatibility.json` needs the new revisions and `game.rs` SHA-256
`996432b9f7f9461fa69c90ff798daf6b89fe3795f379d7c6f401916633296463`; one online
`cargo fetch --locked` for `zune-jpeg`. Not executed here.

**Three slice-1 statements are now false at the new pin** and should be
corrected in `notes.md` when the port re-pins:

- “the pin has no mod key bind or scroll-wheel hook” — `native_wheel`
  (`native_hooks.rs:2601`), `native_mouse_scrolling` (`:2607`), `native_key`
  (`:2569`) and `native_mod_key` (`:9159`) exist.
- “`use_item_on` gives `&ItemStack` and has no component-write path” —
  components can be read/written (`native_components.rs:26,49,74`) and the
  acting player’s carried stack is mutable (`native_hooks.rs:8450`).
- “the pin has no FE hook” — item energy exists (`native_item_energy_buffer`,
  `native_hooks.rs:3093`; helpers `native_item_energy.rs:156,168,180`).

## Deferred features

| ID | Deferred feature (notes) | Best new-pin API (symbol, file:line) | Class | Verdict | Next step |
|---|---|---|---|---|---|
| D1 | Seven other modes + mode switching (`notes.md:40-42`) | `native_use_item_on` `native_hooks.rs:6482`; pure `runtime.rs`; `native_wheel` `:2601`; API 13/1 | usable | **realizable now** | Implement the mode enum + planner in `runtime.rs` first (TDD against `HorizontalRow.java`’s rule); wire mode selection through the wheel and the existing `use_item_on`. No new hook. |
| D2 | Selection/range are per-player in memory (`notes.md:43-45`) | `native_component_types` `native_hooks.rs:2913`; `native_component` `native_components.rs:26`; `native_set_component` `:49`; `native_remove_component` `:74`; `native_inventory_tick` `:7614`; `native_carried_stack` `:8450`; API 65/39 `docs/native-mods.md:825,326` | usable | **realizable now** — a registered `buildinggadgets2:gadget` component persists mode/range/selection on the stack | Register the component type; in `inventory_tick` (selected slot) write it through `carried_stack`; in `use_item_on` read it from the passed `&ItemStack`. E2E: set range, relog, range survives. |
| D3 | No scroll-wheel/key control (`notes.md:26,43-45`) | `native_wheel` `native_hooks.rs:2601`; `native_mouse_scrolling` `:2607` (client, API 122); `native_key` `:2569`; `native_mod_key` `:9159`; `native_input_key` `:2575`; API 1/121/122 | usable | **realizable now** | Replace the `/buildinggadgets2 range` fallback with a wheel/key helper; keep the command as an accessibility fallback. E2E: scroll changes the range and the placed length. |
| D4 | `cleanBlockState` not applied (`notes.md:46-48`) | `native_block_property` `native_hooks.rs:9063`; `native_block_state_property` `native_block_entity_draw.rs:742`; `native_state_with_property` `:8372`; API 20/33 | usable | **realizable now** | Read the state’s properties, drop transient ones (age/lit/waterlogged/open/…), rebuild with `state_with_property`; the transient set is a pure runtime table and a unit test. |
| D5 | `buildinggadgets2:deny` tag not consulted (`notes.md:46-48`) | `native_server_block_in_tag` `native_world_read.rs:69`; `native_server_item_in_tag` `:79`; API 63 `docs/native-mods.md:819` | usable | **realizable now** | In `use_item_on`, call `native_server_block_in_tag(state, "buildinggadgets2:deny")`; refuse the position. Add the tag as content. E2E: denied block is skipped, not built. |
| D6 | Placement is immediate: no queue/retry (`notes.md:49-52`) | `native_server_tick` `native_hooks.rs:2456`; `native_level_tick_post` `:2468`; `native_set_block` `:8464`; `native_can_survive` `:1643`; `native_state_for_placement` `:1727`; `native_update_shape` `:1609`; API 4/23/24/32/76 | usable | **realizable now** | Queue the planned cells in `support.rs`; each `server_tick` writes up to N, retries a failed `can_survive` once, and stops when the supply/energy runs out. E2E: a support placed later in the row is filled on the retry pass. |
| D7 | `BlockItem.place`/`setPlacedBy` and block-entity data not carried (`notes.md:49-52`) | `native_place_block` `native_hooks.rs:6775` is **listen-only**; `native_state_for_placement` `:1727`; `native_block_entity` `:2985`; `native_with_mod_block_entity` `:8724` (mod BEs only) | usable for states and mod BEs; **no generic `setPlacedBy` invocation** | **partial** — stairs/fences/planks work via the computed state; chests/beds/doors and vanilla `setPlacedBy` side effects do not | Compute the state with `state_for_placement`; for a mod BE, let `native_block_entity` create it; for vanilla pairing blocks, document the limitation or exclude them from the gadget’s placeable set. Do not claim parity. |
| D8 | No build animation / render preview (`notes.md:49-52`) | `native_world_drawn` `crates/horizon-client-render/src/gfx/native_render.rs:152` (device/queue/encoder/colour/depth/matrices); `native_render_needs` `:165`; `native_block_entity_submit` `native_hooks.rs:3190`; API 58/71 | usable (raw wgpu draw context); **no ready-made ghost-block helper** | **partial / realizable at high effort** — a mod can draw its own world-space quads, but it owns the pipeline, shader and assets | Defer. If preview is required, prototype a single translucent quad pipeline in `native_world_drawn` behind a config; do not start it in the same slice as D1/D2. |
| D9 | No energy (upstream 500 000 FE / 50 per block) (`notes.md:53-54`) | `native_item_energy_buffer` `native_hooks.rs:3093`; `native_item_energy` `native_item_energy.rs:156`; `insert` `:168`; `extract` `:180`; `native_block_entity_energy` `native_hooks.rs:3066`; `native_energy` `machines/native_energy.rs:137`; API 307/142 `docs/native-mods.md:1492,1139` | usable | **realizable now** — the gadget can hold FE in `custom_data.horizon_energy`, saved/sent with the stack | Declare the item buffer (500 000, transfer limits); charge through `item_energy_insert` on `carried_stack`; charge 50 per placed block in D6. A charger block can use `block_entity_energy` later. E2E: empty gadget places nothing. |
| D10 | No anchor (`notes.md:55-57`) | pure state + D2 component; no hook | usable | **realizable now** | Store the anchor in the gadget component; the pure planner takes it as an input. E2E: anchor far from the player, row starts there. |
| D11 | No undo list (`notes.md:55-57`) | `native_attachments` `native_hooks.rs:8406` (`Holder::Player` / `Level`); `native_set_block` `:8464`; `native_give_stack` `native_server.rs:707`; API 317/24 | usable | **realizable now** | Record `(pos, previous_state)` per placement in a saved attachment; `/buildinggadgets2 undo` restores in reverse and refunds the items. E2E: build 5, undo 5, states restored and items returned. |
| D12 | No bound inventory/AE2/Curios sourcing (`notes.md:55-57`) | `native_container_slots` `machines/native_items.rs:116`; `native_extract_item` `:142`; `native_move_items` `:180`; `native_insert_item` `:9077`; `native_player_stacks` `native_menus.rs:165`; `native_inventory` `native_inventories.rs:164`; API 48/132 | usable | **partial** — bound vanilla containers and the player inventory work; **AE2 is absent** (no AE2 port), and there are no custom Curios slots | Implement an optional bound container by position (extract via `native_extract_item`); fall back to the player inventory. Leave AE2 out; record it as “needs the AE2 port”. |
| D13 | No `place_on_top` / connected-area (`notes.md:55-57`) | pure state/planner; no hook | usable | **realizable now** | Add both as pure planner flags; the e2e covers the toggled row offset. |
| D14 | No GUI/tooltip (`notes.md:55-57`) | `native_tooltip` `native_hooks.rs:6413`; `native_mod_screen_open` `:8968`; `native_screen_render_post` `:3372`; `native_mouse_scrolling` `:2607`; `native_mouse_button_pre` `:2580`; API 12/78/111/122 | usable | **realizable now** | Tooltip first (mode/range/energy); a radial mode picker can be a mod screen or a HUD layer driven by the wheel. E2E: tooltip text changes with mode/range. |
| D15 | No sounds/particles (`notes.md:55-57`) | `native_play_sound` `native_hooks.rs:8193`; `native_play_local_sound` `:8782`; `native_send_particles` `:8233`; API 25/83/11 | usable | **realizable now** | Play the placement sound per cell and a small particle burst on the first/last cell; rate-limit. |
| D16 | No chunk protection (`mayInteract`) (`notes.md:55-57`) | `native_block_lock_check` `native_hooks.rs:4953`; `native_right_click_block` `:6790`; `native_destroy_block` `:6763`; `native_block_spread` `:1339`; `native_explosion_detonate` `:6803`; API 195/21/22/14 | usable for the port’s own claim list; **no single generic `mayInteract` API** | **realizable now for the gadget’s own placements** (a config claim list); **partial** for integrating other protection mods | Add a config claim list checked by the planner before `set_block`. `block_lock_check` is a **container-lock** hook (guests checking a container), not general land protection; use it only for that case. Do not claim protection-mod parity. |
| D17 | Component-bearing stacks are not counted (`notes.md:58-59`) | `native_inventory` `native_inventories.rs:164`; `native_player_stacks` `native_menus.rs:165`; `native_item_container_slots` `native_item_container.rs:166`; existing `native_carried_items` `native_server.rs:641` is plain-only | usable | **realizable now** | Count from `native_inventory`/`native_player_stacks` (full stacks, components included) instead of `native_count_items`; keep `take_items` for plain stacks only. |

## Summary

- **Deferred features audited: 17.**
- **Realizable now in full: 13** — D1–D6, D9–D11, D13–D15, D17.
- **Newly opened by `83ad384f`: 5** — D2 (component writes), D3
  (wheel/scroll/key), D9 (item energy), D14 (mod screens/tooltip), D17
  (full-stack inventory reads). The slice-1 notes explicitly name these as
  absent, so they are genuine re-pin gains.
- **Newly practical/documented: 5** — D4 (state reads), D5 (deny tag), D6
  (tick queue + `set_block`), D11 (attachments), D12 (container helpers).
  These may have had partial support at the old pin, but the new pin documents
  and exposes a direct path; treat them as scope now, not as new hooks.
- **Partial: 4** — D7 (`setPlacedBy`/vanilla BE data), D8 (preview needs a
  hand-written wgpu pipeline), D12 (AE2 absent, no Curios slots), D16 (own
  claims only).
- **Still blocked: 1 feature** — **AE2 sourcing**, inside D12 (no AE2 port and
  no AE2 API; it is a cross-port dependency, not a hook gap). D12 itself is
  partial rather than blocked because vanilla containers and the player
  inventory work.
- **Highest-value next step:** **D2+D3 first** — persist mode/range/selection
  in a registered item component and switch the range with the wheel. It is a
  small, independent slice that removes the command fallback and the relog
  reset, and it is the enabler for the real feature: **D1, the seven modes**.
  After that, D6 (queued placement + retry) and D9 (energy) make the gadget
  behave like upstream; defer D8 (preview) until the modes and persistence are
  verified.

## Gotchas found while reading

- `native_place_block` is a **hook** (listen), not a callable placement
  helper (`native_hooks.rs:6775`). There is no `BlockItem.place` /
  `setPlacedBy` invocation in the API, so D7 cannot be made faithful by
  finding a better symbol; it needs an API request or a documented exclusion
  list.
- `native_set_block` creates a block entity for a **mod** block, but a vanilla
  block’s own `setPlacedBy` (chest pairing, bed halves, door halves) does not
  run. `native_with_mod_block_entity` only reaches mod BEs
  (`native_hooks.rs:8724`).
- `native_item_energy_insert/extract` return a **new single item**; the stack
  given is not changed. Charge through `native_carried_stack` (selected slot)
  or shrink/give the result, as the docs describe (`docs/native-mods.md:1492`).
- `native_carried_items`/`native_count_items` are plain-only
  (`native_server.rs:503-505`); D17 must use `native_inventory` or
  `native_player_stacks`.
- `native_mouse_scrolling` fires only with a player in gameplay and no open
  screen (`docs/native-mods.md:1079`); the wheel-driven radial picker needs its
  own screen/HUD path, not a screen-scroll hook.
- The mode/range component must be written in `inventory_tick` (the only place
  the held stack is mutable); `use_item_on` receives `&ItemStack`, so it can
  read but not persist.
- `native_server_block_in_tag` is the context-free tag read usable from a
  player hook; `native_block_in_tag` needs a live tick context
  (`docs/native-mods.md:249,819`). D5 should use the former.
