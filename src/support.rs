//! Game-side glue for Building Gadgets 2 slice 1 (the building gadget's row).
//!
//! [`crate::runtime`] decides what a click does and every number it uses; this module is
//! the only place that reads the game and writes it back: the clicked block state, the
//! held stack's `buildinggadgets2:gadget_state` component, the inventory that pays for
//! the row, the blocks themselves, and the two replies.
//!
//! The 0.1.0 bundle did the same through the retired native hooks; the mapping is in
//! `notes.md` under `## Migration to the SDK format`.

use mod_api::{
    BlockFace, BlockMetadata, BlockState, BlockUpdateFlags, ClientHost, Command,
    CommandArgumentValue, CommandInvocation, CommandSource, Component, Dist, Flow, Hand, Host,
    HostBlockStates, HostSide, InteractionResult, ItemKey, ItemPart, ItemStack, LevelRef,
    MouseScrollingEvent, PlayerRef, RunModCommandEvent, ServerHost, Tag, Text, UseOnEvent,
};

use crate::runtime::{
    self, COPY_GADGET, DEFAULT_RANGE, DESTROY_GADGET, DestroyFacts, DestroyWorld, EXCHANGE_GADGET,
    GadgetCommand, MAX_RANGE, Mode, PlayerView,
};

/// The persisted state component, upstream's `GadgetNBT` payload on the stack.
pub const COMPONENT: &str = "buildinggadgets2:gadget_state";

/// `buildinggadgets2.messages.invalidblock` (`GadgetBuilding.onShiftAction`): the action-bar
/// line a refused selection prints, verbatim from the mod's own `en_us.json`.
pub const INVALID_BLOCK: &str = "Invalid Block";

/// The main inventory's slots: the hotbar (0 to 8) and then the 27 backpack slots.
/// Equipment and the off hand are not payment, as upstream pays from the main
/// inventory first.
const MAIN_INVENTORY_END: u32 = 36;

/// The gadget's own state, as upstream keeps it in the item's components.
///
/// `selected` is the block state as `/setblock` takes it back
/// (`minecraft:oak_stairs[facing=east]`), which is what vanilla's block-state codec
/// writes into a component — the same value a relog reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GadgetState {
    /// The tool range, 1 to [`MAX_RANGE`].
    pub range: u32,
    /// The block state a sneak click selected, as upstream stores it: the clicked block's state
    /// with its transient properties at the block's default (`cleanBlockState`).
    pub selected: Option<String>,
    /// The active mode, upstream's `GADGET_MODE` component.
    pub mode: Mode,
    /// Upstream's `PLACE_ON_TOP` toggle.
    pub place_on_top: bool,
    /// Slice 0.2.7: the cells the destruction gadget's last sneak click planned. The plan
    /// travels with the stack so `/buildinggadgets2` can name its count; it removes nothing.
    pub destroy_plan: Option<Vec<[i32; 3]>>,
    /// Slice 0.2.8: the same for the exchange gadget — the destinations it would swap,
    /// stored on its own stack. It places and removes nothing (the build stays held).
    pub exchange_plan: Option<Vec<[i32; 3]>>,
    /// Slice 0.2.9: the Copy/Paste gadget's Copy-mode collection, stored on its own
    /// stack. Paste's real write (`BuildingUtils.build`/`exchange`) stays held.
    pub copy_plan: Option<Vec<[i32; 3]>>,
}

impl Default for GadgetState {
    fn default() -> Self {
        Self {
            range: DEFAULT_RANGE,
            selected: None,
            mode: runtime::DEFAULT_MODE,
            place_on_top: false,
            destroy_plan: None,
            exchange_plan: None,
            copy_plan: None,
        }
    }
}

/// The component descriptor: the id the manifest declares and the payload's codec.
pub struct GadgetStateComponent;

impl Component for GadgetStateComponent {
    type Value = Tag;

    fn id(&self) -> &str {
        COMPONENT
    }

    /// A component that is not a compound is not this gadget's state: the stack is
    /// read as a fresh gadget rather than guessed at.
    fn decode(&self, value: &Tag) -> Option<Tag> {
        match value {
            Tag::Compound(_) => Some(value.clone()),
            _ => None,
        }
    }

    fn encode(&self, value: Tag) -> Tag {
        value
    }
}

/// The stack's state. A missing or malformed component is a fresh gadget (default
/// range, no selection), exactly as upstream's `GadgetNBT` defaults.
fn read_state(stack: &ItemStack) -> GadgetState {
    let Some(tag @ Tag::Compound(_)) = stack.get(&GadgetStateComponent) else {
        return GadgetState::default();
    };
    let range = tag
        .get("range")
        .and_then(|tag| match tag {
            Tag::Int(value) => u32::try_from(*value).ok(),
            _ => None,
        })
        .filter(|range| (1..=MAX_RANGE).contains(range))
        .unwrap_or(DEFAULT_RANGE);
    let selected = tag.get("selected").and_then(|tag| match tag {
        Tag::String(value) => Some(value.to_string_lossy()),
        _ => None,
    });
    let mode = tag
        .get("mode")
        .and_then(|tag| match tag {
            Tag::String(value) => Mode::from_id(&value.to_string_lossy()),
            _ => None,
        })
        .unwrap_or(runtime::DEFAULT_MODE);
    let place_on_top = matches!(tag.get("place_on_top"), Some(Tag::Byte(1)));
    let plan = |key: &str| {
        tag.get(key).and_then(|tag| match tag {
            Tag::List(rows) => rows
                .iter()
                .map(|row| match row {
                    Tag::List(parts) => match parts.as_slice() {
                        [Tag::Int(x), Tag::Int(y), Tag::Int(z)] => Some([*x, *y, *z]),
                        _ => None,
                    },
                    _ => None,
                })
                .collect::<Option<Vec<_>>>(),
            _ => None,
        })
    };
    GadgetState {
        range,
        selected,
        mode,
        place_on_top,
        destroy_plan: plan("destroy_plan"),
        exchange_plan: plan("exchange_plan"),
        copy_plan: plan("copy_plan"),
    }
}

/// Write the state into a stack's component patch. A write the stack refuses (an
/// item whose components cannot take it) leaves the gadget as it was.
fn write_state(stack: &mut ItemStack, state: &GadgetState) {
    let range = i32::try_from(state.range).unwrap_or(i32::from(u8::MAX));
    let mut entries = vec![
        ("range", Tag::Int(range)),
        ("mode", Tag::String(state.mode.id().into())),
        ("place_on_top", Tag::Byte(i8::from(state.place_on_top))),
    ];
    if let Some(selected) = &state.selected {
        entries.push(("selected", Tag::String(selected.as_str().into())));
    }
    let rows = |plan: &[[i32; 3]]| {
        Tag::List(
            plan.iter()
                .map(|cell| {
                    Tag::List(
                        cell.iter()
                            .map(|value| Tag::Int(*value))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
        )
    };
    if let Some(plan) = &state.destroy_plan {
        entries.push(("destroy_plan", rows(plan)));
    }
    if let Some(plan) = &state.exchange_plan {
        entries.push(("exchange_plan", rows(plan)));
    }
    if let Some(plan) = &state.copy_plan {
        entries.push(("copy_plan", rows(plan)));
    }
    let _ = stack.set(&GadgetStateComponent, Tag::compound(entries));
}

/// The main-hand gadget the player holds, with the inventory slot it is in. The
/// selected hotbar slot is the main hand's; an off-hand gadget is not read, as the
/// 0.1.0 port's `game.held()` was the main hand.
fn held_stack(host: &dyn ServerHost, player: PlayerRef) -> Option<(u32, ItemStack)> {
    let slot = u32::try_from(player.selected_slot(host)?).ok()?;
    let stack = player
        .inventory_slots(host)?
        .into_iter()
        .find(|(at, _)| *at == slot)
        .map(|(_, stack)| stack)?;
    Some((slot, stack))
}

/// The main-hand building gadget, for the build path.
fn held_gadget(host: &dyn ServerHost, player: PlayerRef) -> Option<(u32, ItemStack)> {
    held_stack(host, player).filter(|(_, stack)| stack.item().id(host) == runtime::GADGET)
}

/// Any of the four gadgets, for the shared range/command paths. The destruction,
/// exchange and Copy/Paste gadgets carry the same `gadget_state` component as the
/// building gadget, so `/buildinggadgets2 range` and the wheel step work with either held.
fn held_any_gadget(host: &dyn ServerHost, player: PlayerRef) -> Option<(u32, ItemStack)> {
    held_stack(host, player).filter(|(_, stack)| {
        let id = stack.item().id(host);
        id == runtime::GADGET
            || id == DESTROY_GADGET
            || id == EXCHANGE_GADGET
            || id == COPY_GADGET
    })
}

/// Replace the held gadget's state: the stack is copied out, its component written,
/// and the slot set back (the component write is not visible on the borrowed stack
/// the hook is handed).
fn write_held(host: &mut dyn ServerHost, player: PlayerRef, state: &GadgetState) -> bool {
    // All four gadgets carry the same component: the destruction/exchange/copy plans
    // travel on their stacks exactly as the building gadget's selection and range do.
    let Some((slot, mut stack)) = held_any_gadget(host, player) else {
        return false;
    };
    write_state(&mut stack, state);
    player.set_inventory_slot(host, slot, Some(stack)).is_ok()
}

/// Upstream's `GadgetUtils.isValidBlockState`: a real block (not air), not
/// unbreakable, not in the `buildinggadgets2:deny` tag, and a fluid only when it is a source.
///
/// The rule (and its tests) lives in [`runtime::selectable`]; this is the read. `block_in_tag`
/// is the host's `TagsHost::block_in_tag` over the world's merged block tag — the package's own
/// `data/buildinggadgets2/tags/block/deny.json` plus every data pack that extends it — and it is
/// `None` only before those tags load, which [`runtime::denied`] treats as "not tagged".
fn selectable(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3], state: BlockState) -> bool {
    let facts = host.block_states();
    let Ok(name) = facts.name(state.block()) else {
        return false;
    };
    if matches!(
        name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    ) {
        return false;
    }
    let fluid = level.fluid_state(host, pos).map(|fluid| runtime::FluidAt {
        amount: u32::from(fluid.amount),
        source: fluid.source,
    });
    runtime::selectable(
        host.block_in_tag(state.block(), runtime::DENY_TAG),
        facts.destroy_speed(state),
        fluid,
    )
}

/// The three queryable clauses of `isValidDestroyBlockState` at one cell. Air is answered by
/// the block's own id (as in [`selectable`]); destroy speed and the deny tag are the stamped
/// `Backend::block_state_destroy_speed` and `Host::block_in_tag` reads. There is no stamped
/// `RenderBlock` query at this pin, so that fourth line is `blocked(no-render-block-surface)`
/// and is deliberately not asked here (see `runtime::NO_RENDER_BLOCK_SURFACE`).
fn destroy_facts(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<DestroyFacts> {
    let state = level.block_state(host, pos)?;
    let facts = host.block_states();
    let air = matches!(
        facts.name(state.block()).ok()?,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    );
    Some(DestroyFacts {
        air,
        destroy_speed: facts.destroy_speed(state),
        deny_tagged: host.block_in_tag(state.block(), runtime::DENY_TAG),
    })
}

/// The planner's world view, reading the game's own tables through the server host.
struct LevelDestroyWorld<'a> {
    host: &'a dyn ServerHost,
    level: LevelRef,
}

impl DestroyWorld for LevelDestroyWorld<'_> {
    fn facts(&mut self, pos: [i32; 3]) -> Option<DestroyFacts> {
        destroy_facts(self.host, self.level, pos)
    }
}

/// The block's name for the selection reply: the state string resolved back to its block, so the
/// line reads `Selected minecraft:oak_stairs` rather than its properties. The status reply prints
/// the stored state string itself.
fn selected_name(host: &dyn ServerHost, text: &str) -> Option<String> {
    let state = host.block_state_id(text)?;
    host.block_states()
        .name(state.block())
        .ok()
        .map(str::to_owned)
}

/// The state a sneak click stores: upstream's `GadgetUtils.setBlockState` →
/// [`runtime::clean_block_state`], through the game's own metadata — the block's default state
/// (`Block.defaultBlockState`), its property list with the value counts that tell upstream's
/// `age` instances apart, and the game's canonical serialization of the result.
///
/// A fact the host cannot answer falls back to the raw state, so a missing table stores what
/// 0.2.5 stored instead of refusing the selection.
fn stored_state(host: &dyn ServerHost, clicked: BlockState) -> Option<String> {
    let facts = host.block_states();
    let source = host.block_state_string(clicked)?;
    let cleaned = host
        .block_state_default(clicked.block())
        .and_then(|state| host.block_state_string(state))
        .and_then(|default| {
            let definition = facts
                .definition(clicked.block())
                .unwrap_or_default()
                .into_iter()
                .map(|property| runtime::PropertyFacts {
                    name: property.name,
                    values: property.values.len(),
                })
                .collect::<Vec<_>>();
            runtime::clean_block_state(&source, &default, &definition)
        })
        .and_then(|text| host.block_state_id(&text))
        .and_then(|state| host.block_state_string(state));
    Some(cleaned.unwrap_or(source))
}

/// How many items of `item` the main inventory holds. Unlike the 0.1.0 port's
/// `native_count_items`, this counts a stack that carries components too.
fn count_items(host: &dyn ServerHost, player: PlayerRef, item: ItemKey) -> u32 {
    player
        .inventory_slots(host)
        .map(|slots| {
            slots
                .into_iter()
                .filter(|(slot, stack)| *slot < MAIN_INVENTORY_END && stack.item() == item)
                .map(|(_, stack)| stack.count())
                .sum()
        })
        .unwrap_or(0)
}

/// Take up to `count` items of `item` from the main inventory, returning how many
/// left it. A slot the game refuses to write stops the take where it is.
fn take_items(host: &mut dyn ServerHost, player: PlayerRef, item: ItemKey, count: u32) -> u32 {
    let Some(slots) = player.inventory_slots(host) else {
        return 0;
    };
    let mut wanted = count;
    let mut taken = 0;
    for (slot, stack) in slots {
        if wanted == 0 {
            break;
        }
        if slot >= MAIN_INVENTORY_END || stack.item() != item {
            continue;
        }
        let take = stack.count().min(wanted);
        let left = stack.count() - take;
        let next = if left == 0 {
            None
        } else {
            let mut next = stack;
            if next.set_count(left).is_err() {
                break;
            }
            Some(next)
        };
        if player.set_inventory_slot(host, slot, next).is_err() {
            break;
        }
        taken += take;
        wanted -= take;
    }
    taken
}

/// The world the planner reads: a cell may be built in when it is replaceable, and
/// the level has it loaded inside the build height (`block_state` answers `None`
/// otherwise, as the 0.1.0 port's own bounds check did).
///
/// The 0.1.0 bundle also asked the game's own `canSurvive` for the state it was about
/// to place. The SDK exposes `canSurvive` only as a `BlockPart` hook a mod answers
/// for *its own* blocks, so a port can no longer ask another block's answer; the gap
/// is in `notes.md`. Placing through `LevelRef::set_block` is what `/setblock` does,
/// and the game's own shape updates drop a state that cannot stay.
///
/// Every destination also passes the gate vanilla applies to the player's own placement
/// ([`runtime::may_edit`]): the clicked block was checked by the game, but the row runs past
/// it, and `LevelRef::set_block` itself checks residency and height only. Without this a
/// permitted click could place a row across a spawn-protection boundary or out of the border.
struct LevelWorld<'a> {
    host: &'a mut dyn ServerHost,
    level: LevelRef,
    rights: runtime::EditRights,
}

impl runtime::World for LevelWorld<'_> {
    fn can_place(&mut self, pos: [i32; 3]) -> bool {
        if !runtime::may_edit(pos, &self.rights) {
            return false;
        }
        let Some(existing) = self.level.block_state(self.host, pos) else {
            return false;
        };
        // The deny tag's rule per destination: a cell whose block the merged
        // `buildinggadgets2:deny` names is never written into. With the carried tag this is
        // implied by the replaceable test (none of its 45 members is replaceable in any state,
        // measured in 26.3); a data pack that adds a replaceable member — the exchange gadget's
        // own rule upstream — makes it decide.
        if runtime::denied(self.host.block_in_tag(existing.block(), runtime::DENY_TAG)) {
            return false;
        }
        self.host.block_states().replaceable(existing) == Some(true)
    }

    /// `Surface`'s rule: the block behind a candidate must be the clicked block's
    /// state. The state string is the codec-faithful identity the selection stores.
    fn same_state(&mut self, a: [i32; 3], b: [i32; 3]) -> bool {
        let Some(one) = self.level.block_state(self.host, a) else {
            return false;
        };
        let Some(two) = self.level.block_state(self.host, b) else {
            return false;
        };
        match (
            self.host.block_state_string(one),
            self.host.block_state_string(two),
        ) {
            (Some(one), Some(two)) => one == two,
            _ => false,
        }
    }
}

/// Write the planned cells, paying one item of the block's kind per placed cell as
/// the 0.1.0 port did: creative pays nothing, a row stops when the supply runs out,
/// and the last block is put back if the take races.
fn place(
    host: &mut dyn ServerHost,
    player: PlayerRef,
    level: LevelRef,
    cells: &[[i32; 3]],
    state: BlockState,
    item: ItemKey,
) {
    let creative = player.is_creative(host).unwrap_or(false);
    // `UPDATE_ALL`: neighbours and clients are told, as `native_set_block` did.
    let flags = BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS;
    let mut budget = if creative {
        u32::MAX
    } else {
        count_items(host, player, item)
    };
    for cell in cells {
        if budget == 0 {
            break;
        }
        if !matches!(level.set_block(host, *cell, state, flags), Ok(true)) {
            break;
        }
        if !creative {
            if take_items(host, player, item, 1) == 0 {
                if let Some(air) = host.block_state_id("minecraft:air") {
                    let _ = level.set_block(host, *cell, air, flags);
                }
                break;
            }
            budget -= 1;
        }
    }
}

/// The player's right to build at a destination: the inputs of [`runtime::may_edit`]. Every
/// unreadable answer refuses, so a stale handle never places.
fn edit_rights(
    host: &dyn ServerHost,
    player: PlayerRef,
    level: LevelRef,
    stack: &ItemStack,
    clicked: [i32; 3],
    face: [i32; 3],
) -> runtime::EditRights {
    let dedicated = host.dist() == Dist::DedicatedServer;
    let view: &dyn Host = host;
    let in_overworld = level
        .dimension()
        .id(view)
        .is_some_and(|id| id == "minecraft:overworld");
    runtime::EditRights {
        may_build: player.may_build(view).unwrap_or(false),
        // Asked where the click landed, not at a made-up position: the engine resolves the
        // position and answers None for one it cannot place in, which refuses.
        may_use_at: player
            .may_use_item_at(view, clicked, block_face(face), stack)
            .unwrap_or(false),
        // `WorldBorder.isWithinBounds` is checked per destination in
        // [`runtime::may_edit`]; the border is the level's own.
        border: level.world_border(view),
        dedicated,
        op: player.permission_level(view).unwrap_or(0) >= runtime::LEVEL_ADMINS,
        spawn_protection: if dedicated {
            spawn_protection_radius(host)
        } else {
            0
        },
        spawn: level.respawn_data(view).map(|data| data.pos),
        in_overworld,
    }
}

/// A runtime face as the game's `BlockFace`.
const fn block_face(face: [i32; 3]) -> BlockFace {
    match face {
        [0, -1, 0] => BlockFace::Down,
        [0, 1, 0] => BlockFace::Up,
        [0, 0, -1] => BlockFace::North,
        [0, 0, 1] => BlockFace::South,
        [-1, 0, 0] => BlockFace::West,
        _ => BlockFace::East,
    }
}

/// The server's `spawn-protection`, read beside the world directory (the server root on a
/// dedicated server). A file or key it does not carry falls back to vanilla's 16.
fn spawn_protection_radius(host: &dyn ServerHost) -> i32 {
    let Some(root) = host
        .world_dir()
        .and_then(|dir| dir.parent().map(std::path::Path::to_path_buf))
    else {
        return runtime::VANILLA_SPAWN_PROTECTION;
    };
    match std::fs::read_to_string(root.join("server.properties")) {
        Ok(text) => runtime::spawn_protection_from(&text),
        Err(_) => runtime::VANILLA_SPAWN_PROTECTION,
    }
}

/// A right click with the gadget on a block: a sneak click selects the clicked
/// state, a plain click builds the selected state's row. The click is claimed in
/// both cases, as upstream cancels the interaction.
fn use_on_block(
    host: &mut dyn ServerHost,
    event: &mut UseOnEvent<'_>,
) -> Option<InteractionResult> {
    let player = host.player(event.player)?;
    if player.is_spectator(host) == Some(true) {
        return None;
    }
    let level = player.level(host)?;
    let state = read_state(event.stack);
    if player.is_secondary_use_active(host).unwrap_or(false) {
        let clicked = level.block_state(host, event.pos)?;
        if !selectable(host, level, event.pos, clicked) {
            // Upstream's `onShiftAction`: the invalid-block line on the action bar, then the
            // base gadget's pass-through, so the gadget keeps what it already had selected.
            let _ = player.action_bar(host, Text::literal(INVALID_BLOCK));
            return None;
        }
        let text = stored_state(host, clicked)?;
        let name = selected_name(host, &text).unwrap_or_else(|| text.clone());
        let selected = GadgetState {
            range: state.range,
            selected: Some(text),
            mode: state.mode,
            place_on_top: state.place_on_top,
            destroy_plan: state.destroy_plan,
            exchange_plan: state.exchange_plan,
            copy_plan: state.copy_plan,
        };
        if !write_held(host, player, &selected) {
            return None;
        }
        let _ = player.action_bar(host, Text::literal(format!("Selected {name}")));
        return Some(InteractionResult::SuccessServer);
    }
    let selected = state.selected.as_deref()?;
    let place_state = host.block_state_id(selected)?;
    let item = host.block_item(place_state.block())?;
    if item.id(host) == "minecraft:air" {
        return None;
    }
    let entity = u32::try_from(event.player_entity_id)
        .ok()
        .and_then(|id| host.entity(id))?;
    let facing = entity
        .rotation(host)
        .map(|[yaw, _pitch]| runtime::facing_from_yaw(yaw))
        .unwrap_or([0, 0, 1]);
    let position = entity.position(host).unwrap_or([0.0, 0.0, 0.0]);
    let view = PlayerView {
        block: entity.block_position(host).unwrap_or([0, 0, 0]),
        y: position[1],
        eye_y: position[1] + entity.eye_height(host).unwrap_or(1.62),
    };
    let cells = {
        let rights = edit_rights(host, player, level, event.stack, event.pos, event.face);
        let mut world = LevelWorld {
            host,
            level,
            rights,
        };
        runtime::build(
            &mut world,
            state.mode,
            &runtime::BuildRequest {
                clicked: event.pos,
                face: event.face,
                facing,
                player: view,
                place_on_top: state.place_on_top,
                range: state.range,
            },
        )
    };
    match cells {
        runtime::Attempt::Cells(cells) => place(host, player, level, &cells, place_state, item),
        // The pin has no placement call a protection mod can cancel per position, so the
        // click writes nothing and says why (see `runtime::placement_hold`). The planned
        // count keeps the border and protection gate observable in game.
        runtime::Attempt::Held { allowed } => {
            let _ = player.action_bar(host, Text::literal(runtime::held_reply(allowed)));
        }
    }
    Some(InteractionResult::SuccessServer)
}

/// A right click with the destruction gadget: a sneak click plans the destination cells and
/// stores the plan on the stack; a plain click writes nothing. The plan is the deliverable — no
/// block is removed, so the build hold stays exactly where the 0.2.4 ruling left it.
fn use_on_destroy(
    host: &mut dyn ServerHost,
    event: &mut UseOnEvent<'_>,
) -> Option<InteractionResult> {
    let player = host.player(event.player)?;
    if player.is_spectator(host) == Some(true) {
        return None;
    }
    let level = player.level(host)?;
    let state = read_state(event.stack);
    if player.is_secondary_use_active(host).unwrap_or(false) {
        // Upstream's `GadgetDestruction.onAction` passes a click on air with no anchor: the
        // plan is centred on a real block, so an air target is not planned.
        let clicked = level.block_state(host, event.pos)?;
        let air = host
            .block_states()
            .name(clicked.block())
            .ok()
            .is_some_and(|name| {
                matches!(
                    name,
                    "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
                )
            });
        if air {
            return None;
        }
        let mut world = LevelDestroyWorld { host, level };
        let cells = runtime::destroy_plan(&mut world, event.pos, state.range);
        let next = GadgetState {
            range: state.range,
            selected: state.selected,
            mode: state.mode,
            place_on_top: state.place_on_top,
            destroy_plan: Some(cells.clone()),
            exchange_plan: state.exchange_plan,
            copy_plan: state.copy_plan,
        };
        if !write_held(host, player, &next) {
            return None;
        }
        let _ = player.action_bar(host, Text::literal(runtime::destroy_reply(cells.len())));
        return Some(InteractionResult::SuccessServer);
    }
    // The hold: no removal call exists that names the acting player and lets a protection
    // mod veto one cell, and this slice is a plan + readout only.
    let _ = player.action_bar(
        host,
        Text::literal(format!(
            "Destruction Gadget: held (blocked: {}) - 0 removed",
            runtime::NO_CANCELLABLE_PLACEMENT
        )),
    );
    Some(InteractionResult::SuccessServer)
}

/// The destruction gadget's behaviour, attached to `buildinggadgets2:gadget_destruction` in
/// `Hooks::register`. As a Part it is asked only for that item.
#[derive(Clone, Copy, Debug, Default)]
pub struct DestroyGadget;

#[mod_api::horizon_part]
impl ItemPart for DestroyGadget {
    fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        if event.hand != Hand::Main {
            return None;
        }
        use_on_destroy(host, event)
    }
}

/// A right click with the exchange gadget: a sneak click plans the cells
/// `BuildingUtils.exchange` would swap and stores the plan on the stack; a plain click
/// writes nothing. The plan is the deliverable — no block is replaced and no item is
/// paid, so the 0.2.4 build hold stays exactly where it was.
fn use_on_exchange(
    host: &mut dyn ServerHost,
    event: &mut UseOnEvent<'_>,
) -> Option<InteractionResult> {
    let player = host.player(event.player)?;
    if player.is_spectator(host) == Some(true) {
        return None;
    }
    let level = player.level(host)?;
    let state = read_state(event.stack);
    if player.is_secondary_use_active(host).unwrap_or(false) {
        // `BaseMode.isExchangingValid` refuses air: a destination plan needs a real
        // block under the crosshair, as the destruction gadget's does.
        let clicked = level.block_state(host, event.pos)?;
        let air = host
            .block_states()
            .name(clicked.block())
            .ok()
            .is_some_and(|name| {
                matches!(
                    name,
                    "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
                )
            });
        if air {
            return None;
        }
        let mut world = LevelDestroyWorld { host, level };
        let cells = runtime::exchange_plan(&mut world, event.pos, state.range);
        let next = GadgetState {
            range: state.range,
            selected: state.selected,
            mode: state.mode,
            place_on_top: state.place_on_top,
            destroy_plan: state.destroy_plan,
            exchange_plan: Some(cells.clone()),
            copy_plan: state.copy_plan,
        };
        if !write_held(host, player, &next) {
            return None;
        }
        let _ = player.action_bar(host, Text::literal(runtime::exchange_reply(cells.len())));
        return Some(InteractionResult::SuccessServer);
    }
    // The hold: no exchange call exists that names the acting player and lets a protection
    // mod veto one cell, and this slice is a plan + readout only.
    let _ = player.action_bar(
        host,
        Text::literal(format!(
            "Exchanging Gadget: held (blocked: {}) - 0 placed, 0 removed",
            runtime::NO_CANCELLABLE_PLACEMENT
        )),
    );
    Some(InteractionResult::SuccessServer)
}

/// The exchange gadget's behaviour, attached to `buildinggadgets2:gadget_exchanging` in
/// `Hooks::register`. As a Part it is asked only for that item.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExchangeGadget;

#[mod_api::horizon_part]
impl ItemPart for ExchangeGadget {
    fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        if event.hand != Hand::Main {
            return None;
        }
        use_on_exchange(host, event)
    }
}

/// A right click with the Copy/Paste gadget: a sneak click collects the Copy-mode
/// destination cells (`Copy.collectWorld`'s valid cells, restricted to the three stamped
/// queries) and stores the plan on the stack; a plain click writes nothing. Upstream's
/// Paste mode hands the stored list to `BuildingUtils.build`/`exchange`, which is the
/// held write — no block is placed, removed or swapped here.
fn use_on_copy(
    host: &mut dyn ServerHost,
    event: &mut UseOnEvent<'_>,
) -> Option<InteractionResult> {
    let player = host.player(event.player)?;
    if player.is_spectator(host) == Some(true) {
        return None;
    }
    let level = player.level(host)?;
    let state = read_state(event.stack);
    if player.is_secondary_use_active(host).unwrap_or(false) {
        // `Copy.collectWorld` needs a real block under the crosshair: its collection
        // is centred on the clicked area, as the other two planners are.
        let clicked = level.block_state(host, event.pos)?;
        let air = host
            .block_states()
            .name(clicked.block())
            .ok()
            .is_some_and(|name| {
                matches!(
                    name,
                    "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
                )
            });
        if air {
            return None;
        }
        let mut world = LevelDestroyWorld { host, level };
        let cells = runtime::copy_plan(&mut world, event.pos, state.range);
        let next = GadgetState {
            range: state.range,
            selected: state.selected,
            mode: state.mode,
            place_on_top: state.place_on_top,
            destroy_plan: state.destroy_plan,
            exchange_plan: state.exchange_plan,
            copy_plan: Some(cells.clone()),
        };
        if !write_held(host, player, &next) {
            return None;
        }
        let _ = player.action_bar(host, Text::literal(runtime::copy_reply(cells.len())));
        return Some(InteractionResult::SuccessServer);
    }
    // The hold: Paste's real write has no call that names the acting player and lets a
    // protection mod veto one cell, and this slice is a plan + readout only.
    let _ = player.action_bar(
        host,
        Text::literal(format!(
            "Copy Paste Gadget: held (blocked: {}) - 0 placed, 0 removed",
            runtime::NO_CANCELLABLE_PLACEMENT
        )),
    );
    Some(InteractionResult::SuccessServer)
}

/// The Copy/Paste gadget's behaviour, attached to the jar's
/// `buildinggadgets2:gadget_copy_paste` in `Hooks::register`. As a Part it is asked
/// only for that item; the Java class `GadgetCopyPaste` is not the shipped id.
#[derive(Clone, Copy, Debug, Default)]
pub struct CopyGadget;

#[mod_api::horizon_part]
impl ItemPart for CopyGadget {
    fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        if event.hand != Hand::Main {
            return None;
        }
        use_on_copy(host, event)
    }
}

/// The building gadget's behaviour, attached to `buildinggadgets2:gadget_building`
/// in `Hooks::register`. Being a Part, the game asks it only for that item.
#[derive(Clone, Copy, Debug, Default)]
pub struct Gadget;

#[mod_api::horizon_part]
impl ItemPart for Gadget {
    /// `ItemStack.useOn`: the gadget's right click on a block, on the side that owns
    /// the click. Answering consumes it; the client half answers nothing, so its
    /// prediction never writes the world.
    fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        if event.hand != Hand::Main {
            return None;
        }
        use_on_block(host, event)
    }
}

/// Declare the state component before content (and so the item's stack codec) loads.
pub fn declare_component(types: &mut Vec<String>) {
    types.push(COMPONENT.to_owned());
}

/// Declare the `/buildinggadgets2` usages: the range and mode controls, the
/// place-on-top setting, and the usage lines. The radial mode wheel is a client
/// screen this slice does not port, so the mode is named on the command line.
pub fn declare_commands(commands: &mut Vec<Command>) {
    commands.push(Command::new("buildinggadgets2"));
    commands.push(Command::new("buildinggadgets2 range"));
    commands.push(Command::new("buildinggadgets2 range <range:integer>"));
    commands.push(Command::new("buildinggadgets2 mode"));
    commands.push(Command::new("buildinggadgets2 mode <mode:word>"));
    commands.push(Command::new("buildinggadgets2 placeontop"));
    commands.push(Command::new("buildinggadgets2 placeontop <state:word>"));
}

/// Rebuild the line the runtime's parser reads from the dispatcher's parsed command:
/// its literal path and the integer argument, which is the only one declared.
fn command_line(command: &CommandInvocation) -> String {
    let mut line = command.literals.join(" ");
    for argument in &command.arguments {
        match &argument.value {
            CommandArgumentValue::Integer(value) => {
                line.push(' ');
                line.push_str(&value.to_string());
            }
            CommandArgumentValue::String(value) => {
                line.push(' ');
                line.push_str(value);
            }
            _ => {}
        }
    }
    line
}

/// Answer on the action bar when a player ran the command, and through the command's
/// own feedback when anything else did (the console, a command block).
fn reply(host: &mut dyn ServerHost, player: Option<PlayerRef>, line: String) {
    let text = Text::literal(line);
    match player {
        Some(player) => {
            let _ = player.action_bar(host, text);
        }
        None => {
            let _ = host.commands().feedback(text);
        }
    }
}

/// `/buildinggadgets2 [range [n]]`: read and store the held gadget's range. The
/// runtime parses the line, so the chat path and the wheel path share one grammar.
pub fn run_command(host: &mut dyn ServerHost, event: &RunModCommandEvent<'_>) -> Flow {
    if event.command.literals.first().map(String::as_str) != Some("buildinggadgets2") {
        return Flow::Continue;
    }
    let Some(parsed) = runtime::parse_command(&command_line(event.command)) else {
        return Flow::Continue;
    };
    let player = match event.source {
        CommandSource::Player { player, .. } => Some(*player),
        _ => None,
    };
    match parsed {
        GadgetCommand::Status => {
            let held = player.and_then(|player| held_any_gadget(host, player));
            let held_id = held.as_ref().map(|(_, stack)| stack.item().id(host));
            let state = held.map_or_else(GadgetState::default, |(_, stack)| read_state(&stack));
            // The destruction, exchange and Copy/Paste gadgets' readouts name their
            // planned cell count (0 before a plan). The building gadget's readout is
            // the state string the component holds, so the cleaned selection stays observable.
            let line = match held_id.as_deref() {
                Some(id) if id == DESTROY_GADGET => runtime::destroy_status_reply(
                    state.range,
                    state.destroy_plan.as_ref().map(Vec::len),
                ),
                Some(id) if id == EXCHANGE_GADGET => runtime::exchange_status_reply(
                    state.range,
                    state.exchange_plan.as_ref().map(Vec::len),
                ),
                Some(id) if id == COPY_GADGET => runtime::copy_status_reply(
                    state.range,
                    state.copy_plan.as_ref().map(Vec::len),
                ),
                _ => runtime::status_reply(
                    state.mode,
                    state.range,
                    state.selected.as_deref(),
                    state.place_on_top,
                ),
            };
            reply(host, player, line);
        }
        GadgetCommand::SetRange(value) => {
            let Some(player) = player else {
                reply(host, None, "Hold a gadget to set its range".to_owned());
                return Flow::Cancel;
            };
            let Some(gadget) = held_any_gadget(host, player) else {
                reply(
                    host,
                    Some(player),
                    "Hold a gadget to set its range".to_owned(),
                );
                return Flow::Cancel;
            };
            let current = read_state(&gadget.1);
            let state = GadgetState {
                range: value,
                selected: current.selected,
                mode: current.mode,
                place_on_top: current.place_on_top,
                destroy_plan: current.destroy_plan,
                exchange_plan: current.exchange_plan,
                copy_plan: current.copy_plan,
            };
            write_held(host, player, &state);
            reply(host, Some(player), runtime::set_range_reply(value));
        }
        GadgetCommand::SetMode(mode) => {
            let Some(player) = player else {
                reply(
                    host,
                    None,
                    "Hold the building gadget to set its mode".to_owned(),
                );
                return Flow::Cancel;
            };
            let Some(gadget) = held_gadget(host, player) else {
                reply(
                    host,
                    Some(player),
                    "Hold the building gadget to set its mode".to_owned(),
                );
                return Flow::Cancel;
            };
            let current = read_state(&gadget.1);
            let state = GadgetState {
                range: current.range,
                selected: current.selected,
                mode,
                place_on_top: current.place_on_top,
                destroy_plan: current.destroy_plan,
                exchange_plan: current.exchange_plan,
                copy_plan: current.copy_plan,
            };
            write_held(host, player, &state);
            reply(host, Some(player), runtime::set_mode_reply(mode));
        }
        GadgetCommand::SetPlaceOnTop(enabled) => {
            let Some(player) = player else {
                reply(
                    host,
                    None,
                    "Hold the building gadget to change its settings".to_owned(),
                );
                return Flow::Cancel;
            };
            let Some(gadget) = held_gadget(host, player) else {
                reply(
                    host,
                    Some(player),
                    "Hold the building gadget to change its settings".to_owned(),
                );
                return Flow::Cancel;
            };
            let current = read_state(&gadget.1);
            let state = GadgetState {
                range: current.range,
                selected: current.selected,
                mode: current.mode,
                place_on_top: enabled,
                destroy_plan: current.destroy_plan,
                exchange_plan: current.exchange_plan,
                copy_plan: current.copy_plan,
            };
            write_held(host, player, &state);
            reply(host, Some(player), runtime::set_place_on_top_reply(enabled));
        }
        GadgetCommand::Invalid => {
            reply(
                host,
                player,
                format!(
                    "Usage: /buildinggadgets2 range <1-{MAX_RANGE}> | mode <{}> | placeontop <on|off>",
                    Mode::ALL.map(Mode::path).join("|")
                ),
            );
        }
    }
    Flow::Cancel
}

/// A wheel notch in gameplay: step the held gadget's range by one per notch, clamp
/// it to 1..=15, and send the same server command the chat path uses, because only
/// the server writes the stack's component. The notch is always taken, so the hotbar
/// does not scroll while the gadget is held.
pub fn scroll(host: &mut dyn ClientHost, event: &mut MouseScrollingEvent) -> Flow {
    let steps = event.accumulated[1];
    if steps == 0 {
        return Flow::Continue;
    }
    let Some(player) = host.local_player() else {
        return Flow::Continue;
    };
    let Some(slot) = player
        .selected_slot(host)
        .and_then(|slot| u32::try_from(slot).ok())
    else {
        return Flow::Continue;
    };
    let Some(stack) = player
        .inventory_slots(host)
        .and_then(|slots| slots.into_iter().find(|(at, _)| *at == slot))
        .map(|(_, stack)| stack)
    else {
        return Flow::Continue;
    };
    let id = stack.item().id(host);
    if id != runtime::GADGET && id != DESTROY_GADGET && id != EXCHANGE_GADGET && id != COPY_GADGET {
        return Flow::Continue;
    }
    let state = read_state(&stack);
    let next = runtime::scroll_range(state.range, steps);
    if next == state.range {
        return Flow::Cancel;
    }
    let _ = host.send_chat(&format!("/buildinggadgets2 range {next}"));
    Flow::Cancel
}
