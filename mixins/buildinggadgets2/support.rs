// Game-side glue for Building Gadgets 2 slice 1 (horizontal row).
// Injected into `crate::game` as `buildinggadgets2_state`; it needs the game,
// so it is not part of the standalone library (only `runtime.rs` is).
//
// A sneak right click selects the clicked block (upstream's `onShiftAction`);
// a plain right click builds the row of that block away from the player,
// taking one item of the block's kind per placed cell. Upstream's item
// component state is replaced by per-player in-memory state because the pin
// cannot write an item component inside a use hook, and its scroll-wheel range
// is replaced by `/buildinggadgets2 range` because the pin has no mod key or
// scroll hook (both recorded in notes.md).
//
// Energy is not modelled: the pin has no FE hook and slice 1 pays items only
// (upstream also charges `baseCost` energy per block; deferred).

use horizon_client::blocks::{Fluid, StateFacts};
use horizon_client::items::ItemStack;
use horizon_client::text::Text;
use horizon_core::position::BlockPos;
use horizon_world::level::flags;
use horizon_world::nbt::Tag;
use horizon_world::registry::BlockStateId;

use crate::game::native_server::{NativeCommand, NativeServerPlayer};

use crate::game::buildinggadgets2::{
    Command, DEFAULT_RANGE, GADGET, MAX_RANGE, World, facing_from_yaw, parse_command, plan,
    scroll_range, set_range_reply, status_reply,
};

/// The gadget's persisted state, on the stack: upstream keeps `gadget_blockstate`
/// and `range` in the item's components. A missing component is a fresh gadget
/// (default range, no selection).
const COMPONENT: &str = "buildinggadgets2:gadget_state";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GadgetState {
    range: u32,
    selected: Option<BlockStateId>,
}

impl Default for GadgetState {
    fn default() -> Self {
        Self {
            range: DEFAULT_RANGE,
            selected: None,
        }
    }
}

fn read_state(game: &crate::game::Game<'_>, stack: &ItemStack) -> GadgetState {
    let Some(tag) = game.native_component(stack, COMPONENT).ok().flatten() else {
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
    let selected = tag
        .get("selected")
        .and_then(|tag| match tag {
            Tag::Int(value) => u32::try_from(*value).ok(),
            _ => None,
        })
        .map(BlockStateId);
    GadgetState { range, selected }
}

fn write_state(game: &crate::game::Game<'_>, stack: &mut ItemStack, state: GadgetState) {
    let tag = Tag::compound([
        ("range", Tag::Int(state.range as i32)),
        (
            "selected",
            Tag::Int(state.selected.map_or(-1, |state| state.0 as i32)),
        ),
    ]);
    if let Err(error) = game.native_set_component(stack, COMPONENT, tag) {
        log::warn!("[buildinggadgets2] component: {error}");
    }
}

/// Write the state to the acting player's held gadget. The stack is moved out
/// so the component helper's `&self` borrow does not overlap `held_mut`.
fn write_held(game: &mut crate::game::Game<'_>, state: GadgetState) {
    let Some(mut stack) = game.held_mut().take() else {
        return;
    };
    if &*stack.item == GADGET {
        write_state(game, &mut stack, state);
    }
    *game.held_mut() = Some(stack);
}

/// Declares the component type before content loads.
pub fn on_component_types(types: &mut Vec<String>) {
    types.push(COMPONENT.to_owned());
}

/// Upstream's `GadgetUtils.isValidBlockState`: a real block (not air), not
/// unbreakable, and a fluid only when it is a source.
fn selectable(facts: &StateFacts) -> bool {
    let air = matches!(
        &*facts.name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    );
    !air && facts.hardness >= 0.0 && (facts.fluid == Fluid::None || facts.fluid_level == 8)
}

/// The state at a position, if the level has it loaded.
fn block_state(game: &crate::game::Game<'_>, [x, y, z]: [i32; 3]) -> Option<BlockStateId> {
    let world = &game.world;
    if !(-30_000_000..30_000_000).contains(&x)
        || !(-30_000_000..30_000_000).contains(&z)
        || !(world.shape.min_y..world.shape.max_y()).contains(&y)
        || !world.loaded(x, z)
    {
        return None;
    }
    Some(world.block(x, y, z))
}

struct GameWorld<'a, 'g> {
    game: &'a mut crate::game::Game<'g>,
    state: BlockStateId,
}

impl World for GameWorld<'_, '_> {
    fn can_place(&mut self, pos: [i32; 3]) -> bool {
        let Some(existing) = block_state(self.game, pos) else {
            return false;
        };
        if !self.game.res.facts().get(existing).replaceable {
            return false;
        }
        let state = self.state;
        let at = BlockPos::new(pos[0], pos[1], pos[2]);
        self.game
            .with_level(|behaviours, level, ticks| behaviours.can_survive(level, ticks, at, state))
    }
}

/// A right click with the building gadget on a block: sneak selects the
/// clicked block, otherwise the clicked face's row is built.
#[allow(private_interfaces)]
pub fn on_use_on(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    pos: [i32; 3],
    face: [i32; 3],
    claimed: &mut bool,
) {
    if &*stack.item != GADGET || stack.count == 0 {
        return;
    }
    let Some(player) = game.native_acting_player() else {
        return;
    };
    // `player.mayBuild()`: spectators build nothing.
    if player.game_mode == 3 {
        return;
    }
    let state = read_state(game, stack);
    let sneaking = game
        .player
        .as_ref()
        .is_some_and(|player| player.shift_key_down);
    if sneaking {
        let Some(clicked_state) = block_state(game, pos) else {
            return;
        };
        let (name, ok) = {
            let facts = game.res.facts().get(clicked_state);
            (facts.name.to_string(), selectable(facts))
        };
        if !ok {
            return;
        }
        write_held(
            game,
            GadgetState {
                range: state.range,
                selected: Some(clicked_state),
            },
        );
        *claimed = true;
        game.native_feedback(Text::literal(format!("Selected {name}")));
        return;
    }
    let Some(selected) = state.selected else {
        return;
    };
    if game.pack.assets.registry.state(selected).is_none() {
        return;
    }
    let item_id = game.res.facts().get(selected).item.to_string();
    if item_id == "minecraft:air" {
        return;
    }
    let range = state.range;
    let facing = game
        .player
        .as_ref()
        .map_or([0, 0, 1], |player| facing_from_yaw(player.yaw));
    let cells = {
        let mut world = GameWorld {
            game: &mut *game,
            state: selected,
        };
        plan(&mut world, pos, face, facing, false, range)
    };
    *claimed = true;
    if cells.is_empty() {
        return;
    }
    let dimension = player.dimension.clone();
    for cell in cells {
        if !game.creative && game.native_count_items(&item_id) == 0 {
            break;
        }
        if !game.native_set_block(&dimension, cell, selected, flags::UPDATE_ALL) {
            break;
        }
        if !game.creative && game.native_take_items(&item_id, 1) == 0 {
            // The supply vanished mid-row: put the last block back.
            if let Some(air) = game.pack.assets.registry.default_state("minecraft:air") {
                game.native_set_block(&dimension, cell, air, flags::UPDATE_ALL);
            }
            break;
        }
    }
}

/// Declares the `/buildinggadgets2` usages: the documented replacement for
/// the missing scroll-wheel range control.
pub fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("buildinggadgets2"));
    commands.push(NativeCommand::new("buildinggadgets2 range"));
    commands.push(NativeCommand::new("buildinggadgets2 range <range:integer>"));
}

/// A player typed a command: answer `/buildinggadgets2 ...` and keep the
/// range on the held gadget's component. Anything else stays unclaimed.
#[allow(private_interfaces)]
pub fn on_perform_command(
    game: &mut crate::game::Game<'_>,
    player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let _ = player;
    let Some(parsed) = parse_command(command) else {
        return;
    };
    *ran = true;
    match parsed {
        Command::Status => {
            let state = game
                .held()
                .map_or_else(GadgetState::default, |stack| read_state(game, stack));
            let selected = state
                .selected
                .filter(|state| game.pack.assets.registry.state(*state).is_some())
                .map(|state| game.res.facts().get(state).name.to_string());
            let reply = status_reply(state.range, selected.as_deref());
            game.native_feedback(Text::literal(reply));
        }
        Command::SetRange(value) => {
            if !game.held().is_some_and(|stack| &*stack.item == GADGET) {
                game.native_feedback(Text::literal("Hold the building gadget to set its range"));
                return;
            }
            let state = game
                .held()
                .map_or_else(GadgetState::default, |stack| read_state(game, stack));
            write_held(
                game,
                GadgetState {
                    range: value,
                    selected: state.selected,
                },
            );
            game.native_feedback(Text::literal(set_range_reply(value)));
        }
        Command::Invalid => {
            let reply = format!("Usage: /buildinggadgets2 range <1-{MAX_RANGE}>");
            game.native_feedback(Text::literal(reply));
        }
    }
}

/// A wheel step in gameplay (API 122): change the held gadget's range by one
/// per notch, clamp it to 1..=15, and send the same server command the chat
/// fallback uses so the persisted component changes on the server. The client
/// cancels the hotbar scroll either way.
#[allow(private_interfaces)]
pub fn on_mouse_scrolling(
    game: &mut crate::game::Game<'_>,
    accumulated: [i32; 2],
    canceled: &mut bool,
) {
    let steps = accumulated[1];
    if steps == 0 {
        return;
    }
    let Some(stack) = game.held() else {
        return;
    };
    if &*stack.item != GADGET {
        return;
    }
    let state = read_state(game, stack);
    let next = scroll_range(state.range, steps);
    if next == state.range {
        *canceled = true;
        return;
    }
    *canceled = true;
    if let Err(error) = game.native_send_chat(&format!("/buildinggadgets2 range {next}")) {
        log::warn!("[buildinggadgets2] wheel range: {error}");
    }
}
