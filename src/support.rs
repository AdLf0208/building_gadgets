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
    BlockMetadata, BlockState, BlockUpdateFlags, ClientHost, Command, CommandArgumentValue,
    CommandInvocation, CommandSource, Component, Flow, Hand, HostBlockStates, HostSide,
    InteractionResult, ItemKey, ItemPart, ItemStack, LevelRef, MouseScrollingEvent, PlayerRef,
    RunModCommandEvent, ServerHost, Tag, Text, UseOnEvent,
};

use crate::runtime::{self, Command as GadgetCommand, DEFAULT_RANGE, MAX_RANGE};

/// The persisted state component, upstream's `GadgetNBT` payload on the stack.
pub const COMPONENT: &str = "buildinggadgets2:gadget_state";

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
    /// The block state a sneak click selected.
    pub selected: Option<String>,
}

impl Default for GadgetState {
    fn default() -> Self {
        Self {
            range: DEFAULT_RANGE,
            selected: None,
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
    GadgetState { range, selected }
}

/// Write the state into a stack's component patch. A write the stack refuses (an
/// item whose components cannot take it) leaves the gadget as it was.
fn write_state(stack: &mut ItemStack, state: &GadgetState) {
    let range = i32::try_from(state.range).unwrap_or(i32::from(u8::MAX));
    let mut entries = vec![("range", Tag::Int(range))];
    if let Some(selected) = &state.selected {
        entries.push(("selected", Tag::String(selected.as_str().into())));
    }
    let _ = stack.set(&GadgetStateComponent, Tag::compound(entries));
}

/// The main-hand gadget the player holds, with the inventory slot it is in. The
/// selected hotbar slot is the main hand's; an off-hand gadget is not read, as the
/// 0.1.0 port's `game.held()` was the main hand.
fn held_gadget(host: &dyn ServerHost, player: PlayerRef) -> Option<(u32, ItemStack)> {
    let slot = u32::try_from(player.selected_slot(host)?).ok()?;
    let stack = player
        .inventory_slots(host)?
        .into_iter()
        .find(|(at, _)| *at == slot)
        .map(|(_, stack)| stack)?;
    (stack.item().id(host) == runtime::GADGET).then_some((slot, stack))
}

/// Replace the held gadget's state: the stack is copied out, its component written,
/// and the slot set back (the component write is not visible on the borrowed stack
/// the hook is handed).
fn write_held(host: &mut dyn ServerHost, player: PlayerRef, state: &GadgetState) -> bool {
    let Some((slot, mut stack)) = held_gadget(host, player) else {
        return false;
    };
    write_state(&mut stack, state);
    player.set_inventory_slot(host, slot, Some(stack)).is_ok()
}

/// Upstream's `GadgetUtils.isValidBlockState`: a real block (not air), not
/// unbreakable, and a fluid only when it is a source.
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
    // A destroy speed of -1 is vanilla's unbreakable block (bedrock, barrier).
    if !facts.destroy_speed(state).is_some_and(|speed| speed >= 0.0) {
        return false;
    }
    match level.fluid_state(host, pos) {
        Some(fluid) if fluid.amount > 0 => fluid.source,
        _ => true,
    }
}

/// The block's name for a status reply: the state string resolved back to its block,
/// so a reply reads `minecraft:oak_stairs`, not its properties.
fn selected_name(host: &dyn ServerHost, text: &str) -> Option<String> {
    let state = host.block_state_id(text)?;
    host.block_states()
        .name(state.block())
        .ok()
        .map(str::to_owned)
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
struct LevelWorld<'a> {
    host: &'a mut dyn ServerHost,
    level: LevelRef,
}

impl runtime::World for LevelWorld<'_> {
    fn can_place(&mut self, pos: [i32; 3]) -> bool {
        let Some(existing) = self.level.block_state(self.host, pos) else {
            return false;
        };
        self.host.block_states().replaceable(existing) == Some(true)
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
            return None;
        }
        let text = host.block_state_string(clicked)?;
        let name = selected_name(host, &text).unwrap_or_else(|| text.clone());
        let selected = GadgetState {
            range: state.range,
            selected: Some(text),
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
    let facing = u32::try_from(event.player_entity_id)
        .ok()
        .and_then(|id| host.entity(id))
        .and_then(|entity| entity.rotation(host))
        .map(|[yaw, _pitch]| runtime::facing_from_yaw(yaw))
        .unwrap_or([0, 0, 1]);
    let cells = {
        let mut world = LevelWorld { host, level };
        runtime::plan(
            &mut world,
            event.pos,
            event.face,
            facing,
            false,
            state.range,
        )
    };
    place(host, player, level, &cells, place_state, item);
    Some(InteractionResult::SuccessServer)
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

/// Declare the `/buildinggadgets2` usages: the range control, and the usage line.
pub fn declare_commands(commands: &mut Vec<Command>) {
    commands.push(Command::new("buildinggadgets2"));
    commands.push(Command::new("buildinggadgets2 range"));
    commands.push(Command::new("buildinggadgets2 range <range:integer>"));
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
            let state = player
                .and_then(|player| held_gadget(host, player))
                .map_or_else(GadgetState::default, |(_, stack)| read_state(&stack));
            let selected = state
                .selected
                .as_deref()
                .and_then(|text| selected_name(host, text));
            let line = runtime::status_reply(state.range, selected.as_deref());
            reply(host, player, line);
        }
        GadgetCommand::SetRange(value) => {
            let Some(player) = player else {
                reply(
                    host,
                    None,
                    "Hold the building gadget to set its range".to_owned(),
                );
                return Flow::Cancel;
            };
            let Some(gadget) = held_gadget(host, player) else {
                reply(
                    host,
                    Some(player),
                    "Hold the building gadget to set its range".to_owned(),
                );
                return Flow::Cancel;
            };
            let state = GadgetState {
                range: value,
                selected: read_state(&gadget.1).selected,
            };
            write_held(host, player, &state);
            reply(host, Some(player), runtime::set_range_reply(value));
        }
        GadgetCommand::Invalid => {
            reply(
                host,
                player,
                format!("Usage: /buildinggadgets2 range <1-{MAX_RANGE}>"),
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
    if stack.item().id(host) != runtime::GADGET {
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
