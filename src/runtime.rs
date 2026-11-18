// Building Gadgets 2 for Horizon — the horizontal-row planner (pure).
//
// Ported from Building Gadgets 2 (Direwolf20-MC/BuildingGadgets2, MIT) for
// Minecraft 1.21.1 NeoForge. The upstream source was read for behaviour only;
// this is a clean-room Rust rewrite.
//
// Slice 1 (building gadget, horizontal row): a right click with the gadget
// builds a row of the selected block away from the player, starting at the
// clicked block and running along the clicked face's horizontal axis, bounded
// by the tool range and the player's supply. Each placed block consumes one
// item of that block's kind. Modes other than the row, the anchor, undo,
// energy and the GUI are later slices (see notes.md).
//
// This file is pure logic so it compiles and unit-tests as the crate's
// standalone library; the game-side glue lives in the bundle's support module.

pub const GADGET: &str = "buildinggadgets2:gadget_building";

/// Upstream's `GadgetNBT.getToolRange` default: one block.
pub const DEFAULT_RANGE: u32 = 1;
/// Upstream's range slider (`ModeRadialMenu`) allows 1..=15.
pub const MAX_RANGE: u32 = 15;

/// A wheel step on the gadget: upstream's slider moves one block per notch and
/// clamps to 1..=15. Positive `steps` (scroll up) increases the range.
#[must_use]
pub fn scroll_range(current: u32, steps: i32) -> u32 {
    (i64::from(current) + i64::from(steps)).clamp(1, i64::from(MAX_RANGE)) as u32
}

/// The horizontal direction a player faces, from their yaw
/// (`Direction.fromYRot`: 0° faces +Z, 90° faces −X).
#[must_use]
pub fn facing_from_yaw(yaw: f32) -> [i32; 3] {
    match (((yaw / 90.0) + 0.5).floor() as i64).rem_euclid(4) {
        0 => [0, 0, 1],
        1 => [-1, 0, 0],
        2 => [0, 0, -1],
        _ => [1, 0, 0],
    }
}

/// The direction the row grows, exactly as `HorizontalRow.collectWorld`
/// computes it: a horizontal face uses the direction opposite the clicked
/// face (away from the player); a top or bottom face uses the player's facing.
#[must_use]
pub fn row_side(face: [i32; 3], facing: [i32; 3]) -> [i32; 3] {
    if face[1] != 0 {
        [facing[0], 0, facing[2]]
    } else {
        [-face[0], 0, -face[2]]
    }
}

/// What the planner reads from the world.
pub trait World {
    /// Whether a block may be placed at `pos`: replaceable, inside the build
    /// height, able to survive there and not blocked by protection. The glue
    /// decides.
    fn can_place(&mut self, pos: [i32; 3]) -> bool;
}

/// The cells one use of the gadget places, in placement order.
///
/// `clicked` is the block that was right-clicked; `face` is its clicked face
/// normal; `facing` is the player's horizontal facing. With `place_on_top`
/// the row starts one block above the clicked block (upstream's toggle).
/// Invalid cells are skipped, not stopped at, as upstream does.
#[must_use]
pub fn plan(
    world: &mut impl World,
    clicked: [i32; 3],
    face: [i32; 3],
    facing: [i32; 3],
    place_on_top: bool,
    range: u32,
) -> Vec<[i32; 3]> {
    let side = row_side(face, facing);
    if range == 0 || side == [0, 0, 0] {
        return Vec::new();
    }
    let start = if place_on_top {
        [clicked[0], clicked[1] + 1, clicked[2]]
    } else {
        clicked
    };
    (0..range)
        .map(|step| {
            let step = step as i32;
            [
                start[0] + side[0] * step,
                start[1] + side[1] * step,
                start[2] + side[2] * step,
            ]
        })
        .filter(|pos| world.can_place(*pos))
        .collect()
}

/// A parsed `/buildinggadgets2` command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// `/buildinggadgets2` or `/buildinggadgets2 range`: report the settings.
    Status,
    /// `/buildinggadgets2 range <n>`: set the range.
    SetRange(u32),
    /// A command in this namespace that the slice does not implement.
    Invalid,
}

/// Parse a command line without its leading slash. `None` when the line is
/// not ours; `Some(Command::Invalid)` when it is ours but malformed.
#[must_use]
pub fn parse_command(line: &str) -> Option<Command> {
    let line = line.trim().trim_start_matches('/');
    let mut words = line.split_whitespace();
    if words.next()? != "buildinggadgets2" {
        return None;
    }
    let Some(verb) = words.next() else {
        return Some(Command::Status);
    };
    if verb != "range" {
        return Some(Command::Invalid);
    }
    let Some(value) = words.next() else {
        return Some(Command::Status);
    };
    if words.next().is_some() {
        return Some(Command::Invalid);
    }
    match value.parse::<u32>() {
        Ok(range) if (1..=MAX_RANGE).contains(&range) => Some(Command::SetRange(range)),
        _ => Some(Command::Invalid),
    }
}

/// The action-bar reply to a status request.
#[must_use]
pub fn status_reply(range: u32, selected: Option<&str>) -> String {
    let selected = selected.unwrap_or("none");
    format!("Building Gadget: range {range}, block {selected}")
}

/// The action-bar reply after a range change.
#[must_use]
pub fn set_range_reply(range: u32) -> String {
    format!("Building Gadget range set to {range}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    struct TestWorld {
        blocked: HashSet<[i32; 3]>,
    }

    impl TestWorld {
        fn new() -> Self {
            TestWorld {
                blocked: HashSet::new(),
            }
        }
        fn block(&mut self, pos: [i32; 3]) {
            self.blocked.insert(pos);
        }
    }

    impl World for TestWorld {
        fn can_place(&mut self, pos: [i32; 3]) -> bool {
            !self.blocked.contains(&pos)
        }
    }

    #[test]
    fn facing_snaps_to_the_four_horizontals() {
        assert_eq!(facing_from_yaw(0.0), [0, 0, 1], "south");
        assert_eq!(facing_from_yaw(90.0), [-1, 0, 0], "west");
        assert_eq!(facing_from_yaw(180.0), [0, 0, -1], "north");
        assert_eq!(facing_from_yaw(270.0), [1, 0, 0], "east");
        assert_eq!(facing_from_yaw(-90.0), [1, 0, 0], "east");
        assert_eq!(facing_from_yaw(359.0), [0, 0, 1], "wraps to south");
        assert_eq!(facing_from_yaw(45.0), [-1, 0, 0], "boundary to west");
    }

    #[test]
    fn a_side_face_builds_away_from_the_player() {
        // Click the south face of a block at z=0 while standing south of it:
        // the row runs north, through the clicked block and beyond.
        let mut world = TestWorld::new();
        world.block([0, 100, 0]);
        let cells = plan(&mut world, [0, 100, 0], [0, 0, 1], [0, 0, 1], false, 6);
        assert_eq!(
            cells,
            vec![
                [0, 100, -1],
                [0, 100, -2],
                [0, 100, -3],
                [0, 100, -4],
                [0, 100, -5]
            ]
        );
    }

    #[test]
    fn a_top_face_uses_the_player_facing() {
        let mut world = TestWorld::new();
        let cells = plan(&mut world, [0, 100, 0], [0, 1, 0], [1, 0, 0], false, 4);
        assert_eq!(
            cells,
            vec![[0, 100, 0], [1, 100, 0], [2, 100, 0], [3, 100, 0]]
        );
    }

    #[test]
    fn a_bottom_face_uses_the_player_facing() {
        let mut world = TestWorld::new();
        let cells = plan(&mut world, [0, 100, 0], [0, -1, 0], [0, 0, -1], false, 3);
        assert_eq!(cells, vec![[0, 100, 0], [0, 100, -1], [0, 100, -2]]);
    }

    #[test]
    fn place_on_top_starts_above_the_clicked_block() {
        let mut world = TestWorld::new();
        let cells = plan(&mut world, [0, 100, 0], [0, 1, 0], [0, 0, 1], true, 3);
        assert_eq!(cells, vec![[0, 101, 0], [0, 101, 1], [0, 101, 2]]);
    }

    #[test]
    fn the_range_caps_the_row() {
        let mut world = TestWorld::new();
        assert_eq!(
            plan(&mut world, [0, 100, 0], [0, 0, 1], [0, 0, 1], false, 1).len(),
            1
        );
        assert_eq!(
            plan(&mut world, [0, 100, 0], [0, 0, 1], [0, 0, 1], false, 15).len(),
            15
        );
        assert!(plan(&mut world, [0, 100, 0], [0, 0, 1], [0, 0, 1], false, 0).is_empty());
    }

    #[test]
    fn an_invalid_cell_is_skipped_not_stopped_at() {
        let mut world = TestWorld::new();
        world.block([0, 100, -2]);
        let cells = plan(&mut world, [0, 100, 0], [0, 0, 1], [0, 0, 1], false, 5);
        assert_eq!(
            cells,
            vec![[0, 100, 0], [0, 100, -1], [0, 100, -3], [0, 100, -4]]
        );
    }

    #[test]
    fn the_default_range_is_one_and_the_slider_stops_at_fifteen() {
        assert_eq!(DEFAULT_RANGE, 1);
        assert_eq!(MAX_RANGE, 15);
    }

    #[test]
    fn commands_are_parsed_in_the_mod_namespace() {
        assert_eq!(parse_command("buildinggadgets2"), Some(Command::Status));
        assert_eq!(
            parse_command("/buildinggadgets2 range"),
            Some(Command::Status)
        );
        assert_eq!(
            parse_command("buildinggadgets2 range 6"),
            Some(Command::SetRange(6))
        );
        assert_eq!(
            parse_command("  /buildinggadgets2   range   15 "),
            Some(Command::SetRange(15))
        );
        assert_eq!(
            parse_command("buildinggadgets2 range 0"),
            Some(Command::Invalid)
        );
        assert_eq!(
            parse_command("buildinggadgets2 range 16"),
            Some(Command::Invalid)
        );
        assert_eq!(
            parse_command("buildinggadgets2 range many"),
            Some(Command::Invalid)
        );
        assert_eq!(
            parse_command("buildinggadgets2 range 2 extra"),
            Some(Command::Invalid)
        );
        assert_eq!(
            parse_command("buildinggadgets2 mode grid"),
            Some(Command::Invalid)
        );
        assert_eq!(parse_command("ultimine shape"), None);
        assert_eq!(parse_command(""), None);
    }

    #[test]
    fn wheel_steps_clamp_to_the_slider() {
        assert_eq!(scroll_range(6, 1), 7);
        assert_eq!(scroll_range(6, -1), 5);
        assert_eq!(scroll_range(1, -1), 1);
        assert_eq!(scroll_range(15, 1), 15);
        assert_eq!(scroll_range(1, 0), 1);
    }

    #[test]
    fn replies_name_the_range_and_selection() {
        assert_eq!(
            status_reply(6, Some("minecraft:stone")),
            "Building Gadget: range 6, block minecraft:stone"
        );
        assert_eq!(
            status_reply(1, None),
            "Building Gadget: range 1, block none"
        );
        assert_eq!(set_range_reply(5), "Building Gadget range set to 5");
    }
}
